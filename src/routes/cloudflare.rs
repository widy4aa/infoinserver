use axum::{Json, http::StatusCode, extract::Extension};
use serde::{Deserialize, Serialize};
use std::process::Command;
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;

#[derive(Serialize, Deserialize, Debug)]
pub struct CloudflareStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub service_active: bool,
    pub running: bool,
    pub auth_cert_exists: bool,
    pub config_exists: bool,
    pub tunnel_uuid: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateTunnelRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct LoginStartResponse {
    pub url: Option<String>,
    pub message: String,
}

#[derive(Serialize)]
pub struct LoginStatusResponse {
    pub authenticated: bool,
}

pub async fn get_cloudflare_status(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<CloudflareStatus>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    // 1. Cek apakah cloudflared terinstall
    let installed = Command::new("which")
        .arg("cloudflared")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    if !installed {
        return Ok(Json(CloudflareStatus {
            installed: false,
            version: None,
            service_active: false,
            running: false,
            auth_cert_exists: false,
            config_exists: false,
            tunnel_uuid: None,
        }));
    }

    // 2. Ambil versi
    let version = Command::new("cloudflared")
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| {
            let raw = String::from_utf8_lossy(&out.stdout).to_string();
            // format: "cloudflared version 2026.x.x (built ...)"
            raw.split_whitespace().nth(2).map(|s| s.to_string())
        });

    // 3. Cek service status via systemctl
    let service_active = Command::new("systemctl")
        .args(["is-active", "cloudflared"])
        .output()
        .map(|out| {
            let status = String::from_utf8_lossy(&out.stdout).trim().to_string();
            status == "active"
        })
        .unwrap_or(false);

    // 4. Cek apakah process berjalan (pgrep)
    let running = Command::new("pgrep")
        .arg("-x")
        .arg("cloudflared")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    // 5. Cek cert.pem (auth status)
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let cert_path = format!("{}/.cloudflared/cert.pem", home);
    let auth_cert_exists = std::path::Path::new(&cert_path).exists();

    // 6. Baca config.yml untuk ambil tunnel UUID
    let config_path = "/etc/cloudflared/config.yml";
    let tunnel_uuid = sudo_exec(&password, &["cat", config_path])
        .ok()
        .and_then(|out| {
            if !out.status.success() {
                return None;
            }
            let content = String::from_utf8_lossy(&out.stdout).to_string();
            // Cari baris "tunnel: <uuid>"
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("tunnel:") {
                    let uuid = trimmed.trim_start_matches("tunnel:").trim().to_string();
                    if !uuid.is_empty() {
                        return Some(uuid);
                    }
                }
            }
            None
        });
        
    let config_exists = tunnel_uuid.is_some();

    Ok(Json(CloudflareStatus {
        installed: true,
        version,
        service_active,
        running,
        auth_cert_exists,
        config_exists,
        tunnel_uuid,
    }))
}

pub async fn install_cloudflared(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    // Unduh ke /tmp
    let _ = Command::new("wget")
        .args(["-q", "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64", "-O", "/tmp/cloudflared"])
        .output();
        
    // Pindahkan dan beri permisi eksekusi menggunakan sudo
    let _ = sudo_exec(&password, &["mv", "/tmp/cloudflared", "/usr/local/bin/cloudflared"]);
    let output = sudo_exec(&password, &["chmod", "+x", "/usr/local/bin/cloudflared"])
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

pub async fn create_tunnel(
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<CreateTunnelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    // Validasi nama tunnel (hanya huruf, angka, dash)
    if payload.name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Tunnel name cannot be empty".to_string()));
    }
    if !payload.name.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err((StatusCode::BAD_REQUEST, "Tunnel name may only contain letters, numbers, and dashes".to_string()));
    }

    let output = Command::new("cloudflared")
        .args(["tunnel", "create", &payload.name])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create tunnel: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);

    if output.status.success() {
        // Coba extract UUID dari output
        // Format output: "Created tunnel <name> with id <uuid>"
        let uuid = combined
            .lines()
            .find(|l| l.contains("with id"))
            .and_then(|l| l.split("with id").nth(1))
            .map(|s| s.trim().to_string());
            
        if let Some(ref tunnel_id) = uuid {
            // Pindahkan kredensial ke /etc/cloudflared agar systemd dapat membacanya
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            let source_json = format!("{}/.cloudflared/{}.json", home, tunnel_id);
            let dest_json = format!("/etc/cloudflared/{}.json", tunnel_id);
            
            // Buat direktori /etc/cloudflared jika belum ada
            let _ = sudo_exec(&password, &["mkdir", "-p", "/etc/cloudflared"]);
            let _ = sudo_exec(&password, &["cp", &source_json, &dest_json]);
            
            // Buat default config.yml dengan kredensial ini
            let default_config = format!(
                "tunnel: {}\ncredentials-file: {}\ningress:\n  - service: http_status:404\n",
                tunnel_id, dest_json
            );
            
            // Tulis file config.yml dengan aman (tulis ke /tmp lalu mv)
            let tmp_path = format!("/tmp/config_{}.yml", tunnel_id);
            let _ = std::fs::write(&tmp_path, default_config);
            let _ = sudo_exec(&password, &["mv", &tmp_path, "/etc/cloudflared/config.yml"]);
        }

        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Tunnel '{}' created successfully!", payload.name),
            "uuid": uuid,
            "output": combined.trim()
        })))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, combined))
    }
}

/// Mulai proses cloudflared tunnel login secara async,
/// capture URL otorisasi dari output dan kembalikan ke frontend.
pub async fn start_tunnel_login() -> Result<Json<LoginStartResponse>, (StatusCode, String)> {
    use std::io::BufRead;
    use std::process::Stdio;

    // Jalankan cloudflared tunnel login, capture stdout+stderr
    let mut child = std::process::Command::new("cloudflared")
        .args(["tunnel", "login"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to start login: {}", e)))?;

    // Baca output line by line selama max 15 detik, cari URL https://dash.cloudflare.com
    let stderr = child.stderr.take().ok_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to capture stderr".to_string())
    })?;

    let reader = std::io::BufReader::new(stderr);
    let mut login_url: Option<String> = None;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);

    for line in reader.lines() {
        if std::time::Instant::now() > deadline {
            break;
        }
        if let Ok(line) = line {
            // URL otorisasi biasanya berisi "https://dash.cloudflare.com/"
            if line.contains("https://dash.cloudflare.com") || line.contains("https://cloudflare.com") {
                // Cari URL dalam line
                if let Some(start) = line.find("https://") {
                    let url_raw = &line[start..];
                    // Ambil sampai whitespace
                    let url = url_raw.split_whitespace().next().unwrap_or("").to_string();
                    if !url.is_empty() {
                        login_url = Some(url);
                        break;
                    }
                }
            }
        }
    }

    // Biarkan child process terus berjalan di background (tunggu user authorize)
    // Tidak di-wait agar process tetap hidup
    std::mem::forget(child);

    if let Some(url) = login_url {
        Ok(Json(LoginStartResponse {
            url: Some(url),
            message: "Authorization URL generated. Open it in your browser and select your domain.".to_string(),
        }))
    } else {
        Ok(Json(LoginStartResponse {
            url: None,
            message: "cloudflared login started but URL not captured. Check if cloudflared is already authenticated.".to_string(),
        }))
    }
}

/// Poll endpoint untuk cek apakah cert.pem sudah ada (login selesai)
pub async fn check_login_status() -> Result<Json<LoginStatusResponse>, (StatusCode, String)> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let cert_path = format!("{}/.cloudflared/cert.pem", home);
    let authenticated = std::path::Path::new(&cert_path).exists();
    Ok(Json(LoginStatusResponse { authenticated }))
}

pub async fn stop_cloudflare_tunnel(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    
    // Stop service cloudflared (managed tunnel)
    let _ = sudo_exec(&password, &["systemctl", "stop", "cloudflared"]);

    // Juga matikan process manual jika ada
    let _ = sudo_exec(&password, &["pkill", "-x", "cloudflared"]);

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Cloudflare tunnel service stopped"
    })))
}
