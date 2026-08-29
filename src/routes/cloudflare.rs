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
    pub tunnel_name: Option<String>,
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
            tunnel_name: None,
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
    
    // 7. Cari nama tunnel berdasarkan UUID via 'cloudflared tunnel list' (Jalankan tanpa sudo agar cert.pem user lokal terbaca)
    let mut tunnel_name = None;
    if let Some(ref uuid) = tunnel_uuid {
        if let Ok(out) = Command::new("cloudflared").args(["tunnel", "list"]).output() {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                // Format stdout biasanya: ID  NAME  CREATED  CONNECTIONS
                for line in stdout.lines() {
                    if line.contains(uuid) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        // Asumsi bagian ke-2 adalah NAME (index 1)
                        if parts.len() >= 2 {
                            tunnel_name = Some(parts[1].to_string());
                        }
                        break;
                    }
                }
            }
        }
    }

    Ok(Json(CloudflareStatus {
        installed: true,
        version,
        service_active,
        running,
        auth_cert_exists,
        config_exists,
        tunnel_uuid,
        tunnel_name,
    }))
}

pub async fn install_cloudflared(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    // 1. Buat temporary directory yang aman via bash (mktemp)
    let temp_dir_out = sudo_exec(&password, &["mktemp", "-d"])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create temp dir: {}", e)))?;
    let temp_dir = String::from_utf8_lossy(&temp_dir_out.stdout).trim().to_string();

    if temp_dir.is_empty() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create temp directory".to_string()));
    }

    let download_path = format!("{}/cloudflared", temp_dir);

    // 2. Download via wget ke temp_dir. Wajib periksa exit status.
    let wget_out = Command::new("wget")
        .args(["-q", "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64", "-O", &download_path])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to execute wget: {}", e)))?;

    if !wget_out.status.success() {
        // Bersihkan temp dir jika gagal
        let _ = sudo_exec(&password, &["rm", "-rf", &temp_dir]);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to download cloudflared binary from GitHub".to_string()));
    }
        
    // 3. Pindahkan dan beri permisi eksekusi menggunakan sudo
    let _ = sudo_exec(&password, &["mv", &download_path, "/usr/local/bin/cloudflared"]);
    let output = sudo_exec(&password, &["chmod", "+x", "/usr/local/bin/cloudflared"])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Execution failed: {}", e)))?;

    // 4. Bersihkan folder sementara
    let _ = sudo_exec(&password, &["rm", "-rf", &temp_dir]);

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

    // Pastikan 1 device hanya punya 1 tunnel
    let config_path = "/etc/cloudflared/config.yml";
    if std::path::Path::new(config_path).exists() || sudo_exec(&password, &["cat", config_path]).is_ok() {
        return Err((StatusCode::BAD_REQUEST, "A tunnel is already configured on this device. Please delete it first before creating a new one.".to_string()));
    }

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
            
            // Buat default config.yml dengan kredensial ini (menggunakan serde_yaml di fungsi lain lebih baik, tapi string ini cukup aman untuk initial)
            let default_config = format!(
                "tunnel: {}\ncredentials-file: {}\ningress:\n  - service: http_status:404\n",
                tunnel_id, dest_json
            );
            
            // Tulis file config.yml dengan aman menggunakan 'sudo sh -c' tanpa membuat file statis di /tmp
            let write_cmd = format!("echo '{}' > /etc/cloudflared/config.yml", default_config);
            let _ = sudo_exec(&password, &["sh", "-c", &write_cmd]);
            
            // Otomatis jalankan service install dan start
            let _ = sudo_exec(&password, &["cloudflared", "--config", "/etc/cloudflared/config.yml", "service", "install"]);
            let _ = sudo_exec(&password, &["systemctl", "enable", "cloudflared"]);
            let _ = sudo_exec(&password, &["systemctl", "start", "cloudflared"]);
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

/// Menghapus tunnel secara permanen dari server lokal dan Cloudflare.
pub async fn delete_tunnel(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    // 1. Baca tunnel UUID sebelum dihapus (dari config.yml)
    let config_path = "/etc/cloudflared/config.yml";
    let tunnel_uuid = sudo_exec(&password, &["cat", config_path])
        .ok()
        .and_then(|out| {
            if !out.status.success() { return None; }
            let content = String::from_utf8_lossy(&out.stdout).to_string();
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("tunnel:") {
                    let uuid = trimmed.trim_start_matches("tunnel:").trim().to_string();
                    if !uuid.is_empty() { return Some(uuid); }
                }
            }
            None
        });

    if let Some(uuid) = tunnel_uuid {
        // 2. Stop and disable service systemd
        let _ = sudo_exec(&password, &["systemctl", "stop", "cloudflared"]);
        let _ = sudo_exec(&password, &["systemctl", "disable", "cloudflared"]);
        let _ = sudo_exec(&password, &["pkill", "-x", "cloudflared"]); // make sure

        // 3. Delete tunnel from Cloudflare (Requires cert.pem to be available/logged in)
        // Kita juga tambahkan argumen --force agar jika ada rute yang menyangkut, bisa dibypass.
        let del_out = sudo_exec(&password, &["cloudflared", "tunnel", "delete", "-f", &uuid]);
        if let Ok(out) = del_out {
            if !out.status.success() {
                // Jangan error out, bisa jadi tunnel sudah tidak ada di cloudflare
                tracing::warn!("Warning during cloudflare tunnel delete: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        // 4. Bersihkan file lokal
        let _ = sudo_exec(&password, &["rm", "-f", config_path]);
        let credential_path = format!("/etc/cloudflared/{}.json", uuid);
        let _ = sudo_exec(&password, &["rm", "-f", &credential_path]);
        
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Tunnel and configurations deleted successfully."
        })))
    } else {
        Err((StatusCode::BAD_REQUEST, "No active tunnel configuration found to delete.".to_string()))
    }
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

/// Mendapatkan logs dari journalctl untuk service cloudflared
pub async fn get_cloudflare_logs(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    
    // Ambil 100 baris terakhir dari journalctl untuk cloudflared
    let output = sudo_exec(&password, &["journalctl", "-u", "cloudflared", "-n", "100", "--no-pager", "-o", "cat"])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read logs: {}", e)))?;

    let logs = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Split berdasarkan baris
    let log_lines: Vec<String> = logs.lines().map(|s| s.to_string()).collect();

    Ok(Json(serde_json::json!({
        "logs": log_lines
    })))
}
