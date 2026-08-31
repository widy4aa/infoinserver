use axum::{Json, http::StatusCode, extract::{Extension, State}};
use serde::{Deserialize, Serialize};
use std::process::Command;
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;
use crate::AppState;

#[derive(Serialize)]
pub struct Fail2BanStatus {
    pub installed: bool,
    pub active: bool,
    pub jails: Vec<JailStatus>,
}

#[derive(Serialize)]
pub struct JailStatus {
    pub name: String,
    pub banned_ips: Vec<String>,
}

#[derive(Deserialize)]
pub struct UnbanRequest {
    pub jail: String,
    pub ip: String,
}

pub async fn get_status_handler(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Fail2BanStatus>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    // Cek instalasi
    let is_installed = Command::new("which")
        .arg("fail2ban-client")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_installed {
        return Ok(Json(Fail2BanStatus {
            installed: false,
            active: false,
            jails: vec![],
        }));
    }

    // Cek service aktif
    let is_active = Command::new("systemctl")
        .args(["is-active", "fail2ban"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false);

    if !is_active {
        return Ok(Json(Fail2BanStatus {
            installed: true,
            active: false,
            jails: vec![],
        }));
    }

    // Ambil list jails
    let mut jails_status = Vec::new();
    if let Ok(out) = sudo_exec(&password, &["fail2ban-client", "status"]) {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            // Parsing output: "`- Jail list:    sshd, nginx"
            if let Some(jail_line) = stdout.lines().find(|l| l.contains("Jail list:")) {
                if let Some(list_part) = jail_line.split(':').nth(1) {
                    let jail_names: Vec<&str> = list_part.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                    
                    // Untuk setiap jail, ambil banned IPs
                    for j in jail_names {
                        if let Ok(j_out) = sudo_exec(&password, &["fail2ban-client", "status", j]) {
                            if j_out.status.success() {
                                let j_stdout = String::from_utf8_lossy(&j_out.stdout).to_string();
                                // Parsing output: "|- Banned IP list:    192.168.1.100 10.0.0.5"
                                let mut banned_ips = Vec::new();
                                if let Some(banned_line) = j_stdout.lines().find(|l| l.contains("Banned IP list:")) {
                                    if let Some(ips_part) = banned_line.split(':').nth(1) {
                                        banned_ips = ips_part.split_whitespace().map(|s| s.to_string()).collect();
                                    }
                                }
                                jails_status.push(JailStatus {
                                    name: j.to_string(),
                                    banned_ips,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Json(Fail2BanStatus {
        installed: true,
        active: true,
        jails: jails_status,
    }))
}

pub async fn unban_ip_handler(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<UnbanRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    if payload.jail.is_empty() || payload.ip.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Jail and IP are required".to_string()));
    }

    let out = sudo_exec(&password, &["fail2ban-client", "set", &payload.jail, "unbanip", &payload.ip])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to unban: {}", e)))?;

    if out.status.success() {
        crate::routes::logs::log_activity(&state.db_pool, "WARNING", "Fail2Ban Unban", &format!("Unbanned IP {} from jail {}", payload.ip, payload.jail)).await;
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Successfully unbanned IP {} from jail {}", payload.ip, payload.jail)
        })))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Unban failed: {}", err)))
    }
}

pub async fn install_handler(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    // Deteksi package manager
    let is_apt = Command::new("which").arg("apt-get").output().map(|o| o.status.success()).unwrap_or(false);
    let is_pacman = Command::new("which").arg("pacman").output().map(|o| o.status.success()).unwrap_or(false);

    let (cmd, args) = if is_apt {
        ("apt-get", vec!["install", "-y", "fail2ban"])
    } else if is_pacman {
        ("pacman", vec!["-S", "--noconfirm", "fail2ban"])
    } else {
        return Err((StatusCode::NOT_IMPLEMENTED, "Unsupported package manager. Please install manually.".to_string()));
    };

    let out = sudo_exec(&password, &[cmd, args[0], args[1], args[2]])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to install: {}", e)))?;

    if out.status.success() {
        // Enable & Start service
        let _ = sudo_exec(&password, &["systemctl", "enable", "fail2ban"]);
        let _ = sudo_exec(&password, &["systemctl", "start", "fail2ban"]);
        
        crate::routes::logs::log_activity(&state.db_pool, "INFO", "Fail2Ban Install", "Fail2Ban has been installed and started").await;
        
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Fail2Ban installed successfully."
        })))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Install failed: {}", err)))
    }
}