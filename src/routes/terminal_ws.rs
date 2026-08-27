use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::{StreamExt, SinkExt}; // Penting untuk socket.split() dan stream reading

pub async fn terminal_ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_terminal_socket)
}

async fn handle_terminal_socket(mut socket: WebSocket) {
    // 1. Buat sistem PTY baru
    let pty_system = NativePtySystem::default();
    
    // Set ukuran default (bisa diupdate via WS nanti jika mau)
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

    // 2. Jalankan shell interaktif standar (misal /bin/bash)
    let mut cmd = CommandBuilder::new("/bin/bash");
    // Penting untuk lingkungan terminal interaktif
    cmd.env("TERM", "xterm-256color"); 
    
    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => {
            let _ = socket.send(Message::Text(format!("Failed to spawn shell: {}", e).into())).await;
            return;
        }
    };
    
    // Kita tak butuh akses langsung ke slave lagi dari proses ini
    drop(pair.slave);

    // 3. Setup Reader & Writer asinkron dari PTY
    // portable-pty mengembalikan reader/writer blocking (std::io).
    // Kita harus wrap ke dalam tokio agar tidak memblokir runtime asinkron kita.
    let pty_reader = pair.master.try_clone_reader().unwrap();
    let mut tokio_pty_reader = tokio::task::spawn_blocking(move || {
        let mut std_reader = pty_reader;
        let mut buffer = [0u8; 1024];
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
        
        std::thread::spawn(move || {
            loop {
                match std::io::Read::read(&mut std_reader, &mut buffer) {
                    Ok(0) => break, // EOF
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

    let pty_writer = Arc::new(Mutex::new(pair.master.take_writer().unwrap()));

    // 4. Jembatan komunikasi WebSocket <-> PTY
    // Split socket jadi sender (kirim ke browser) & receiver (terima ketikan browser)
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Task 1: Baca dari PTY -> Kirim ke WebSocket (Tampilan terminal)
    let mut send_task = tokio::spawn(async move {
        while let Some(bytes) = tokio_pty_reader.recv().await {
            // xterm.js dapat menerima binary langsung, tapi teks lebih mudah di debug.
            // PTY mengirimkan ANSI escape codes yang utuh untuk xterm.js
            if ws_sender.send(Message::Binary(bytes.into())).await.is_err() {
                break; // Socket ditutup
            }
        }
    });

    // Task 2: Baca ketikan dari WebSocket -> Tulis ke PTY (Input pengguna)
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
                _ => {} // Abaikan ping/pong
            }
        }
    });

    // Tunggu sampai salah satu task selesai (berarti koneksi putus atau shell exit)
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    // Bersihkan proses anak (shell) jika WS mati
    let _ = child.kill();
    let _ = child.wait();
}