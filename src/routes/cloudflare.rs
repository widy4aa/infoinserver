use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tokio::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct CloudflareStatus {
    pub installed: bool,
    pub running: bool,
    pub active_tunnels: Vec<String>,
}

#[derive(Deserialize)]
pub struct QuickTunnelRequest {
    pub port: String,
}

#[derive(Deserialize)]
pub struct ManagedTunnelRequest {
    pub token: String,
}

pub async fn get_cloudflare_status() -> Result<Json<CloudflareStatus>, (StatusCode, String)> {
    let installed = Command::new("which")
        .arg("cloudflared")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    if !installed {
        return Ok(Json(CloudflareStatus {
            installed: false,
            running: false,
            active_tunnels: vec![],
        }));
    }

    let running = Command::new("pgrep")
        .arg("-x")
        .arg("cloudflared")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    // Ambil log untuk mencari URL aktif (quick tunnel)
    // Cloudflared menyimpan log di stderr jika running manual, atau syslog jika service
    let mut active_tunnels = Vec::new();
    if running {
        // Ini asusmi sangat sederhana. Quick tunnel memprint URL ke stderr.
        // Jika jalan di background via script kita, kita simpan lognya di /tmp/cloudflared.log
        if let Ok(log) = fs::read_to_string("/tmp/cloudflared.log").await {
            for line in log.lines() {
                if line.contains("https://") && line.contains(".trycloudflare.com") {
                    let parts: Vec<&str> = line.split("https://").collect();
                    if parts.len() > 1 {
                        let url_part: Vec<&str> = parts[1].split_whitespace().collect();
                        if !url_part.is_empty() {
                            let url = format!("https://{}", url_part[0]);
                            if !active_tunnels.contains(&url) {
                                active_tunnels.push(url);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Json(CloudflareStatus {
        installed: true,
        running,
        active_tunnels,
    }))
}

pub async fn install_cloudflared() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Kita asumsikan distro Linux 64-bit
    // Unduh dan install ke /usr/local/bin
    let cmd = "wget -q https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -O /tmp/cloudflared && sudo mv /tmp/cloudflared /usr/local/bin/cloudflared && sudo chmod +x /usr/local/bin/cloudflared";
    
    let output = Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Execution failed: {}", e)))?;

    if output.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "cloudflared installed successfully!"
        })))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, String::from_utf8_lossy(&output.stderr).to_string()))
    }
}

pub async fn start_quick_tunnel(Json(payload): Json<QuickTunnelRequest>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Validasi port
    if !payload.port.chars().all(|c| c.is_ascii_digit()) {
        return Err((StatusCode::BAD_REQUEST, "Invalid port".to_string()));
    }

    // Kill existing cloudflared quick tunnel first
    let _ = Command::new("pkill").arg("-x").arg("cloudflared").output();

    // Jalankan cloudflared tunnel --url http://localhost:PORT
    // Log diarahkan ke /tmp/cloudflared.log agar kita bisa membaca URL .trycloudflare
    let cmd = format!("nohup cloudflared tunnel --url http://localhost:{} > /tmp/cloudflared.log 2>&1 &", payload.port);
    
    let output = Command::new("bash")
        .arg("-c")
        .arg(&cmd)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to spawn tunnel: {}", e)))?;

    if output.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Quick tunnel starting in background..."
        })))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to start tunnel".to_string()))
    }
}

pub async fn start_managed_tunnel(Json(payload): Json<ManagedTunnelRequest>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if payload.token.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Token cannot be empty".to_string()));
    }

    // Kill existing cloudflared first
    let _ = Command::new("pkill").arg("-x").arg("cloudflared").output();

    // Instal sebagai service agar permanen (memerlukan sudo)
    let cmd = format!("sudo cloudflared service install {}", payload.token);
    
    let output = Command::new("bash")
        .arg("-c")
        .arg(&cmd)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to install tunnel service: {}", e)))?;

    if output.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Managed Tunnel service installed and started successfully!"
        })))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, String::from_utf8_lossy(&output.stderr).to_string()))
    }
}

pub async fn stop_cloudflare_tunnel() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Matikan manual process
    let _ = Command::new("pkill").arg("-x").arg("cloudflared").output();
    
    // Matikan service (jika managed)
    let _ = Command::new("sudo").args(["systemctl", "stop", "cloudflared"]).output();

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Tunnels stopped"
    })))
}