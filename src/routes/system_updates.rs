use axum::{
    Json, http::StatusCode, extract::Extension, extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use serde::Serialize;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;
use crate::AppState;

#[derive(Serialize)]
pub struct UpdateInfo {
    pub manager: String, // "apt" or "pacman"
    pub updatable_count: usize,
    pub details: Vec<String>,
}

async fn check_apt(password: &str) -> Result<UpdateInfo, String> {
    // Jalankan apt update lebih dulu (diam-diam, blocking tidak masalah karena hanya check)
    let _ = tokio::task::spawn_blocking({
        let p = password.to_string();
        move || sudo_exec(&p, &["apt-get", "update"])
    }).await;

    let out = tokio::task::spawn_blocking({
        let p = password.to_string();
        move || sudo_exec(&p, &["apt", "list", "--upgradable"])
    }).await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    if out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let mut details = Vec::new();
        for line in stdout.lines() {
            if line.contains("upgradable from") {
                details.push(line.to_string());
            }
        }
        Ok(UpdateInfo {
            manager: "apt".to_string(),
            updatable_count: details.len(),
            details,
        })
    } else {
        Err("apt list failed".to_string())
    }
}

async fn check_pacman(password: &str) -> Result<UpdateInfo, String> {
    let is_installed = std::process::Command::new("which").arg("checkupdates").output()
        .map(|o| o.status.success()).unwrap_or(false);
    
    if !is_installed {
        return Ok(UpdateInfo {
            manager: "pacman".to_string(),
            updatable_count: 0,
            details: vec!["'checkupdates' script (from pacman-contrib) is not installed. Cannot check safely.".to_string()],
        });
    }

    let out = tokio::task::spawn_blocking({
        let p = password.to_string();
        move || sudo_exec(&p, &["checkupdates"])
    }).await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let mut details = Vec::new();
    for line in stdout.lines() {
        if !line.trim().is_empty() {
            details.push(line.trim().to_string());
        }
    }
    
    Ok(UpdateInfo {
        manager: "pacman".to_string(),
        updatable_count: details.len(),
        details,
    })
}

pub async fn check_updates_handler(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<UpdateInfo>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    let is_apt = std::process::Command::new("which").arg("apt-get").output()
        .map(|o| o.status.success()).unwrap_or(false);
    let is_pacman = std::process::Command::new("which").arg("pacman").output()
        .map(|o| o.status.success()).unwrap_or(false);

    if is_apt {
        match check_apt(&password).await {
            Ok(info) => Ok(Json(info)),
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
        }
    } else if is_pacman {
        match check_pacman(&password).await {
            Ok(info) => Ok(Json(info)),
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
        }
    } else {
        Err((StatusCode::NOT_IMPLEMENTED, "Unsupported package manager".to_string()))
    }
}

pub async fn upgrade_ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    Extension(auth): Extension<AuthUser>,
) -> Response {
    let password = auth.0.pwd.clone();
    let pool = state.db_pool.clone();
    ws.on_upgrade(move |socket| handle_upgrade_ws(socket, password, pool))
}

async fn handle_upgrade_ws(mut socket: WebSocket, password: String, pool: sqlx::SqlitePool) {
    let is_apt = std::process::Command::new("which").arg("apt-get").output()
        .map(|o| o.status.success()).unwrap_or(false);
    let is_pacman = std::process::Command::new("which").arg("pacman").output()
        .map(|o| o.status.success()).unwrap_or(false);

    let (cmd, cmd_args) = if is_apt {
        ("sudo", vec!["-S", "apt-get", "upgrade", "-y"])
    } else if is_pacman {
        ("sudo", vec!["-S", "pacman", "-Syu", "--noconfirm"])
    } else {
        let _ = socket.send(Message::Text("Unsupported package manager.".into())).await;
        return;
    };

    let _ = socket.send(Message::Text(
        format!("Executing: {} {}\n", cmd, cmd_args.join(" ")).into()
    )).await;

    let mut child = match Command::new(cmd)
        .args(&cmd_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = socket.send(Message::Text(
                format!("[ERROR] Failed to spawn upgrade process: {}", e).into()
            )).await;
            return;
        }
    };

    // Inject sudo password ke stdin
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        let _ = stdin.write_all(format!("{}\n", password).as_bytes()).await;
    }

    // Ambil STDOUT dan STDERR agar keduanya bisa di-stream ke WebSocket
    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            let _ = socket.send(Message::Text("[ERROR] Failed to capture stdout.".into())).await;
            let _ = child.kill().await;
            return;
        }
    };
    let stderr = match child.stderr.take() {
        Some(s) => s,
        None => {
            let _ = socket.send(Message::Text("[ERROR] Failed to capture stderr.".into())).await;
            let _ = child.kill().await;
            return;
        }
    };

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // Stream stdout dan stderr secara bersamaan ke WebSocket
    // apt-get sering menulis output penting ke stderr (progress, errors)
    loop {
        tokio::select! {
            // Baca dari stdout
            line = stdout_reader.next_line() => {
                match line {
                    Ok(Some(log_line)) => {
                        if socket.send(Message::Text(log_line.into())).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => break, // stdout EOF
                    Err(_) => break,
                }
            }
            // Baca dari stderr
            line = stderr_reader.next_line() => {
                match line {
                    Ok(Some(log_line)) if !log_line.trim().is_empty() => {
                        // Kirim stderr dengan prefix agar mudah dibedakan
                        let prefixed = if log_line.contains("ERROR") || log_line.contains("error") {
                            format!("[ERR] {}", log_line)
                        } else {
                            log_line
                        };
                        if socket.send(Message::Text(prefixed.into())).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) => {} // baris kosong, skip
                    Err(_) => {} // ignore stderr errors
                }
            }
            // Deteksi jika client menutup koneksi
            msg = socket.recv() => {
                if let Some(Ok(Message::Close(_))) | None = msg {
                    break;
                }
            }
        }
    }

    // Tunggu proses benar-benar selesai sebelum menutup koneksi
    let _ = child.wait().await;

    crate::routes::logs::log_activity(
        &pool, "WARNING", "System Update",
        "Executed OS package upgrade via Dashboard"
    ).await;

    // Kirim pesan selesai
    let _ = socket.send(Message::Text("\n--- Upgrade Process Finished ---".into())).await;
}
