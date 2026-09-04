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

#[derive(Deserialize)]
pub struct BanRequest {
    pub jail: String,
    pub ip: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JailConfig {
    pub name: String,
    pub enabled: bool,
    pub port: Option<String>,
    pub logpath: Option<String>,
    pub filter: Option<String>,
    pub maxretry: Option<String>,
    pub bantime: Option<String>,
    pub findtime: Option<String>,
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

pub async fn ban_ip_handler(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<BanRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    if payload.jail.is_empty() || payload.ip.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Jail and IP are required".to_string()));
    }

    let out = sudo_exec(&password, &["fail2ban-client", "set", &payload.jail, "banip", &payload.ip])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to ban: {}", e)))?;

    if out.status.success() {
        crate::routes::logs::log_activity(&state.db_pool, "WARNING", "Fail2Ban Manual Ban", &format!("Manually banned IP {} in jail {}", payload.ip, payload.jail)).await;
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Successfully banned IP {} in jail {}", payload.ip, payload.jail)
        })))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Ban failed: {}", err)))
    }
}

pub async fn install_handler(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    let is_apt = Command::new("which").arg("apt-get").output().map(|o| o.status.success()).unwrap_or(false);
    let is_pacman = Command::new("which").arg("pacman").output().map(|o| o.status.success()).unwrap_or(false);

    let (cmd, args) = if is_apt {
        ("apt-get", vec!["install", "-y", "fail2ban"])
    } else if is_pacman {
        ("pacman", vec!["-S", "--noconfirm", "fail2ban"])
    } else {
        return Err((StatusCode::NOT_IMPLEMENTED, "Unsupported package manager.".to_string()));
    };

    let out = sudo_exec(&password, &[cmd, args[0], args[1], args[2]])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to install: {}", e)))?;

    if out.status.success() {
        let _ = sudo_exec(&password, &["systemctl", "enable", "fail2ban"]);
        let _ = sudo_exec(&password, &["systemctl", "start", "fail2ban"]);
        crate::routes::logs::log_activity(&state.db_pool, "INFO", "Fail2Ban Install", "Fail2Ban has been installed and started").await;
        Ok(Json(serde_json::json!({"status": "success", "message": "Fail2Ban installed successfully."})))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Install failed: {}", err)))
    }
}

pub async fn get_logs_handler(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    
    let out = sudo_exec(&password, &["tail", "-n", "100", "/var/log/fail2ban.log"])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read logs: {}", e)))?;

    if out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let logs: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();
        Ok(Json(logs))
    } else {
        Ok(Json(vec![]))
    }
}

pub async fn get_jails_config_handler(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<JailConfig>>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    let config_path = "/etc/fail2ban/jail.local";
    
    let check_exists = sudo_exec(&password, &["cat", config_path]);
    
    let content = match check_exists {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => {
            let default_conf = "[DEFAULT]\nbantime = 10m\nfindtime = 10m\nmaxretry = 5\n\n[sshd]\nenabled = true\nport = ssh\nlogpath = %(sshd_log)s\nbackend = %(sshd_backend)s\n";
            let cmd = format!("echo '{}' > {}", default_conf.replace("'", "'\\''"), config_path);
            let _ = sudo_exec(&password, &["sh", "-c", &cmd]);
            default_conf.to_string()
        }
    };

    let mut jails = Vec::new();
    let mut current_jail: Option<JailConfig> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if let Some(j) = current_jail.take() {
                if j.name != "DEFAULT" { jails.push(j); }
            }
            let name = trimmed.trim_matches(|c| c == '[' || c == ']').to_string();
            current_jail = Some(JailConfig {
                name, enabled: false, port: None, logpath: None, filter: None, maxretry: None, bantime: None, findtime: None,
            });
        } else if let Some(ref mut j) = current_jail {
            if let Some(idx) = trimmed.find('=') {
                let key = trimmed[..idx].trim().to_lowercase();
                let value = trimmed[idx+1..].trim().to_string();
                match key.as_str() {
                    "enabled" => j.enabled = value.to_lowercase() == "true",
                    "port" => j.port = Some(value),
                    "logpath" => j.logpath = Some(value),
                    "filter" => j.filter = Some(value),
                    "maxretry" => j.maxretry = Some(value),
                    "bantime" => j.bantime = Some(value),
                    "findtime" => j.findtime = Some(value),
                    _ => {}
                }
            }
        }
    }
    
    if let Some(j) = current_jail {
        if j.name != "DEFAULT" { jails.push(j); }
    }
    Ok(Json(jails))
}

pub async fn save_jail_config_handler(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<JailConfig>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    let config_path = "/etc/fail2ban/jail.local";
    
    if payload.name.is_empty() || payload.name.to_lowercase() == "default" {
        return Err((StatusCode::BAD_REQUEST, "Invalid jail name".to_string()));
    }

    let out = sudo_exec(&password, &["cat", config_path]).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let content = String::from_utf8_lossy(&out.stdout).to_string();
    
    let mut new_lines = Vec::new();
    let mut in_target_jail = false;
    let mut jail_found = false;

    let mut jail_block = format!("[{}]\nenabled = {}\n", payload.name, payload.enabled);
    if let Some(p) = &payload.port { if !p.is_empty() { jail_block.push_str(&format!("port = {}\n", p)); } }
    if let Some(p) = &payload.logpath { if !p.is_empty() { jail_block.push_str(&format!("logpath = {}\n", p)); } }
    if let Some(p) = &payload.filter { if !p.is_empty() { jail_block.push_str(&format!("filter = {}\n", p)); } }
    if let Some(p) = &payload.maxretry { if !p.is_empty() { jail_block.push_str(&format!("maxretry = {}\n", p)); } }
    if let Some(p) = &payload.bantime { if !p.is_empty() { jail_block.push_str(&format!("bantime = {}\n", p)); } }
    if let Some(p) = &payload.findtime { if !p.is_empty() { jail_block.push_str(&format!("findtime = {}\n", p)); } }

    for line in content.lines() {
        if line.trim().starts_with('[') && line.trim().ends_with(']') {
            let name = line.trim().trim_matches(|c| c == '[' || c == ']').to_string();
            if name == payload.name {
                in_target_jail = true;
                jail_found = true;
                new_lines.push(jail_block.clone());
                continue;
            } else {
                in_target_jail = false;
            }
        }
        if !in_target_jail { new_lines.push(line.to_string()); }
    }

    if !jail_found {
        new_lines.push(String::new());
        new_lines.push(jail_block);
    }

    let new_content = new_lines.join("\n") + "\n";
    let escaped_content = new_content.replace("'", "'\\''");
    let cmd = format!("echo '{}' > {}", escaped_content, config_path);
    
    let write_out = sudo_exec(&password, &["sh", "-c", &cmd])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write config: {}", e)))?;

    if write_out.status.success() {
        let _ = sudo_exec(&password, &["systemctl", "restart", "fail2ban"]);
        crate::routes::logs::log_activity(&state.db_pool, "INFO", "Fail2Ban Config", &format!("Updated jail configuration for {}", payload.name)).await;
        Ok(Json(serde_json::json!({"status": "success", "message": format!("Jail '{}' saved", payload.name)})))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to write config".to_string()))
    }
}

/// Ambil daftar filter yang tersedia dari /etc/fail2ban/filter.d/
pub async fn get_filters_handler(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    let out = sudo_exec(&password, &["ls", "/etc/fail2ban/filter.d/"])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to list filters: {}", e)))?;

    if out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let filters: Vec<String> = stdout
            .lines()
            .filter(|l| l.ends_with(".conf"))
            .map(|l| l.replace(".conf", "").trim().to_string())
            .collect();
        Ok(Json(filters))
    } else {
        Ok(Json(vec![]))
    }
}

/// Hapus jail dari jail.local
pub async fn delete_jail_handler(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    axum::extract::Path(jail_name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    let config_path = "/etc/fail2ban/jail.local";

    if jail_name.is_empty() || jail_name.to_lowercase() == "default" {
        return Err((StatusCode::BAD_REQUEST, "Cannot delete DEFAULT jail".to_string()));
    }

    let out = sudo_exec(&password, &["cat", config_path])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let content = String::from_utf8_lossy(&out.stdout).to_string();

    let mut new_lines = Vec::new();
    let mut in_target = false;
    let mut found = false;

    for line in content.lines() {
        if line.trim().starts_with('[') && line.trim().ends_with(']') {
            let name = line.trim().trim_matches(|c| c == '[' || c == ']').to_string();
            if name == jail_name {
                in_target = true;
                found = true;
                continue;
            } else {
                in_target = false;
            }
        }
        if !in_target {
            new_lines.push(line.to_string());
        }
    }

    if !found {
        return Err((StatusCode::NOT_FOUND, format!("Jail '{}' not found in config", jail_name)));
    }

    let new_content = new_lines.join("\n") + "\n";
    let escaped = new_content.replace("'", "'\\''");
    let cmd = format!("echo '{}' > {}", escaped, config_path);
    let write_out = sudo_exec(&password, &["sh", "-c", &cmd])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if write_out.status.success() {
        let _ = sudo_exec(&password, &["systemctl", "restart", "fail2ban"]);
        crate::routes::logs::log_activity(&state.db_pool, "WARNING", "Fail2Ban Delete Jail", &format!("Deleted jail: {}", jail_name)).await;
        Ok(Json(serde_json::json!({"status": "success", "message": format!("Jail '{}' deleted", jail_name)})))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete jail from config".to_string()))
    }
}

/// Reset Fail2Ban ke default: hapus jail.local + restart service
pub async fn reset_fail2ban(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    let password2 = password.clone();

    // 1. Hapus jail.local
    let out = tokio::task::spawn_blocking(move || {
        sudo_exec(&password, &["rm", "-f", "/etc/fail2ban/jail.local"])
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete jail.local: {}", err)));
    }

    // 2. Restart fail2ban agar perubahan berlaku
    let _ = tokio::task::spawn_blocking(move || {
        sudo_exec(&password2, &["systemctl", "restart", "fail2ban"])
    }).await;

    crate::routes::logs::log_activity(
        &state.db_pool, "WARNING", "Fail2Ban Reset",
        "jail.local deleted and fail2ban restarted (reset to default)"
    ).await;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Fail2Ban reset to default. jail.local removed and service restarted."
    })))
}