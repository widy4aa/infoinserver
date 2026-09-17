use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Extension,
    response::IntoResponse,
};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::{StreamExt, SinkExt};
use crate::auth::jwt_middleware::AuthUser;

pub async fn terminal_ws_handler(
    ws: WebSocketUpgrade,
    Extension(auth): Extension<AuthUser>,
) -> impl IntoResponse {
    let username = auth.0.sub.clone();
    let password = auth.0.pwd.clone();
    ws.on_upgrade(move |socket| handle_terminal_socket(socket, username, password))
}

async fn handle_terminal_socket(mut socket: WebSocket, username: String, password: String) {
    let pty_system = NativePtySystem::default();

    let size = PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = match pty_system.openpty(size) {
        Ok(p) => p,
        Err(e) => {
            let _ = socket.send(Message::Text(format!("Failed to open PTY: {}", e).into())).await;
            return;
        }
    };

    // Spawn bash sebagai user target menggunakan sudo -u.
    // Set PATH eksplisit agar portable-pty spawn menemukan sudo di lokasi yang benar.
    let mut cmd = CommandBuilder::new("sudo");
    cmd.args([
        "-S",           // baca password dari stdin
        "-u", &username,
        "--",
        "bash", "--login",
    ]);
    cmd.env("TERM", "xterm-256color");
    cmd.env("SUDO_PROMPT", ""); // kosongkan prompt sudo agar tidak ada teks "Password:"
    // PATH eksplisit — portable-pty spawn dengan PATH minimal yang tidak include /usr/bin
    cmd.env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => {
            let _ = socket.send(Message::Text(format!("Failed to spawn shell: {}", e).into())).await;
            return;
        }
    };

    drop(pair.slave);

    let pty_reader = pair.master.try_clone_reader().unwrap();
    let pty_writer = Arc::new(Mutex::new(pair.master.take_writer().unwrap()));

    // Inject password ke sudo via stdin (-S flag)
    // Kirim segera sebelum sudo sempat print prompt apapun
    {
        let mut writer = pty_writer.lock().await;
        let _ = std::io::Write::write_all(&mut *writer, format!("{}\n", password).as_bytes());
        let _ = std::io::Write::flush(&mut *writer);
    }

    // Setup reader async — dengan drain noise awal di sisi Rust
    // Semua output sebelum prompt shell ($/#) akan di-drain dan tidak dikirim ke browser.
    // Setelah prompt terdeteksi, kirim ESC sequence clear-screen lalu teruskan output normal.
    let (noise_done_tx, noise_done_rx) = tokio::sync::oneshot::channel::<()>();
    let pty_writer_for_clear = Arc::clone(&pty_writer);

    let mut tokio_pty_reader = tokio::task::spawn_blocking(move || {
        let mut std_reader = pty_reader;
        let mut buffer = [0u8; 1024];
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

        std::thread::spawn(move || {
            loop {
                match std::io::Read::read(&mut std_reader, &mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.blocking_send(buffer[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        rx
    }).await.unwrap();

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Task 1: PTY → WebSocket
    // Fase 1 (noise drain): buffer semua output, deteksi prompt, lalu kirim clear screen
    // Fase 2 (normal): teruskan semua output ke browser
    // noise_done_tx di-wrap Option agar bisa di-consume sekali saja di dalam loop
    let mut noise_done_tx = Some(noise_done_tx);

    let mut send_task = tokio::spawn(async move {
        let mut noise_drained = false;
        let mut accumulated = Vec::<u8>::new();

        while let Some(bytes) = tokio_pty_reader.recv().await {
            if !noise_drained {
                accumulated.extend_from_slice(&bytes);

                let text = String::from_utf8_lossy(&accumulated);
                let stripped = strip_ansi(&text);

                if stripped.contains("$ ") || stripped.ends_with("$ ")
                    || stripped.contains("# ") || stripped.ends_with("# ")
                    || stripped.ends_with("$") || stripped.ends_with("#")
                {
                    noise_drained = true;
                    accumulated.clear();

                    let clear_seq = b"\x1b[2J\x1b[H".to_vec();
                    if ws_sender.send(Message::Binary(clear_seq.into())).await.is_err() {
                        break;
                    }

                    if let Some(tx) = noise_done_tx.take() {
                        let _ = tx.send(());
                    }
                    continue;
                }

                if accumulated.len() > 8192 {
                    noise_drained = true;
                    accumulated.clear();
                    let clear_seq = b"\x1b[2J\x1b[H".to_vec();
                    let _ = ws_sender.send(Message::Binary(clear_seq.into())).await;
                    if let Some(tx) = noise_done_tx.take() {
                        let _ = tx.send(());
                    }
                }

                continue;
            }

            if ws_sender.send(Message::Binary(bytes.into())).await.is_err() {
                break;
            }
        }
    });

    // Setelah noise drain selesai, kirim `clear` ke PTY agar prompt muncul ulang di top
    tokio::spawn(async move {
        if noise_done_rx.await.is_ok() {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            let mut writer = pty_writer_for_clear.lock().await;
            // Kirim `clear` + Enter ke shell untuk refresh prompt
            let _ = std::io::Write::write_all(&mut *writer, b"clear\n");
            let _ = std::io::Write::flush(&mut *writer);
        }
    });

    // Task 2: WebSocket → PTY
    let writer_clone = Arc::clone(&pty_writer);
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Text(t) => {
                    let mut writer = writer_clone.lock().await;
                    let _ = std::io::Write::write_all(&mut *writer, t.as_bytes());
                    let _ = std::io::Write::flush(&mut *writer);
                },
                Message::Binary(b) => {
                    let mut writer = writer_clone.lock().await;
                    let _ = std::io::Write::write_all(&mut *writer, &b);
                    let _ = std::io::Write::flush(&mut *writer);
                },
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    let _ = child.kill();
    let _ = child.wait();
}

/// Strip ANSI escape codes dari string untuk deteksi prompt
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip escape sequence: ESC [ ... m  atau ESC [ ... H dll
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // consume sampai huruf (command char)
                for ch in chars.by_ref() {
                    if ch.is_ascii_alphabetic() { break; }
                }
            } else {
                // ESC + single char
                chars.next();
            }
        } else {
            result.push(c);
        }
    }
    result
}
