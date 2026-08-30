use axum::{Json, http::StatusCode, extract::Extension};
use serde::{Deserialize, Serialize};
use std::process::Command;
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;

const CONFIG_PATH: &str = "/etc/cloudflared/config.yml";

/// Representasi ingress rule dari config.yml yang dikirim ke frontend
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IngressRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    pub service: String,
    // Field baru untuk deteksi CNAME di frontend
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cname_active: Option<bool>,
}

/// Representasi config.yml yang diparsing
#[derive(Serialize, Deserialize, Debug)]
pub struct LocalTunnelConfig {
    pub tunnel: Option<String>,
    #[serde(rename = "credentials-file", alias = "credentials_file")]
    pub credentials_file: Option<String>,
    pub ingress: Vec<IngressRule>,
}

#[derive(Deserialize)]
pub struct AddRouteRequest {
    pub tunnel_name: String,
    pub hostname: String,
    pub service: String,
}

#[derive(Deserialize)]
pub struct DeleteRouteRequest {
    pub hostname: String,
}

#[derive(Deserialize)]
pub struct RegisterDnsRequest {
    pub tunnel_name: String,
    pub hostname: String,
}

/// Baca dan parse /etc/cloudflared/config.yml
pub async fn get_local_config(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<LocalTunnelConfig>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    match read_config_file(&password) {
        Ok(raw) => match parse_config(&raw) {
            Ok(mut config) => {
                // Lakukan pengecekan CNAME untuk setiap hostname secara parallel/sequential
                for rule in &mut config.ingress {
                    if let Some(ref host) = rule.hostname {
                        // Jalankan command 'host -t cname' untuk cek DNS CNAME
                        let check_cmd = Command::new("host")
                            .args(["-t", "cname", host])
                            .output();
                        
                        let is_active = if let Ok(out) = check_cmd {
                            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                            // Jika DNS mengarah ke cfargotunnel.com, artinya aktif
                            out.status.success() && (stdout.contains("cfargotunnel.com") || stdout.contains("alias for"))
                        } else {
                            false
                        };
                        rule.cname_active = Some(is_active);
                    }
                }
                Ok(Json(config))
            },
            Err(e) => Err(e),
        },
        Err(e) => {
            if e.0 == StatusCode::NOT_FOUND {
                // Jika file tidak ada, kembalikan config kosong, jangan 404/500
                Ok(Json(LocalTunnelConfig {
                    tunnel: None,
                    credentials_file: None,
                    ingress: vec![],
                }))
            } else {
                Err(e)
            }
        }
    }
}

/// Tambah route baru:
/// 1) Daftarkan DNS CNAME via `cloudflared tunnel route dns`
/// 2) Update config.yml
/// 3) Restart service
pub async fn add_local_route(
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<AddRouteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    // Validasi input
    if payload.hostname.is_empty() || payload.service.is_empty() || payload.tunnel_name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "hostname, service, and tunnel_name must not be empty".to_string()));
    }

    // 1. Daftarkan DNS CNAME ke Cloudflare
    let dns_output = sudo_exec(&password, &["cloudflared", "tunnel", "route", "dns", &payload.tunnel_name, &payload.hostname])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to register DNS: {}", e)))?;

    let dns_stdout = String::from_utf8_lossy(&dns_output.stdout).to_string();
    let dns_stderr = String::from_utf8_lossy(&dns_output.stderr).to_string();
    let dns_combined = format!("{}{}", dns_stdout, dns_stderr);

    // Lanjut meski DNS gagal (mungkin sudah ada), tapi log hasilnya
    let dns_msg = if dns_output.status.success() {
        format!("DNS registered: {}", dns_combined.trim())
    } else {
        format!("DNS registration warning: {}", dns_combined.trim())
    };

    // 2. Baca config lama
    let raw = match read_config_file(&password) {
        Ok(r) => r,
        Err(e) => {
            if e.0 == StatusCode::NOT_FOUND {
                // If it's missing, it's an error because the tunnel configuration should have been created
                return Err((StatusCode::BAD_REQUEST, "Configuration file /etc/cloudflared/config.yml is missing. Please create the tunnel first.".to_string()));
            } else {
                return Err(e);
            }
        }
    };
    
    let mut config = parse_config(&raw)?;

    // Cek apakah hostname sudah ada
    let already_exists = config.ingress.iter().any(|r| {
        r.hostname.as_deref() == Some(&payload.hostname)
    });
    if already_exists {
        return Err((StatusCode::BAD_REQUEST, format!("Route for '{}' already exists in config", payload.hostname)));
    }

    // 3. Tambah rule baru sebelum catch-all
    let new_rule = IngressRule {
        hostname: Some(payload.hostname.clone()),
        service: payload.service.clone(),
        cname_active: None,
    };
    // Pisahkan catch-all dan rules biasa
    let mut normal_rules: Vec<IngressRule> = config.ingress.into_iter()
        .filter(|r| r.hostname.is_some())
        .collect();
    normal_rules.push(new_rule);
    // Tambahkan catch-all di akhir
    normal_rules.push(IngressRule { hostname: None, service: "http_status:404".to_string(), cname_active: None });
    config.ingress = normal_rules;

    // 4. Tulis kembali ke config.yml
    let new_yaml = build_config_yaml(&config)?;
    write_config_file(&password, &new_yaml)?;

    // 5. Restart service
    restart_service_internal(&password)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": format!("Route '{}' -> '{}' added and service restarted.", payload.hostname, payload.service),
        "dns_result": dns_msg
    })))
}

/// Hapus route dari config dan restart service
pub async fn delete_local_route(
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<DeleteRouteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    if payload.hostname.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "hostname must not be empty".to_string()));
    }

    let raw = read_config_file(&password)?;
    let mut config = parse_config(&raw)?;

    let before_len = config.ingress.len();
    config.ingress.retain(|r| {
        r.hostname.as_deref() != Some(&payload.hostname)
    });

    if config.ingress.len() == before_len {
        return Err((StatusCode::NOT_FOUND, format!("No route found for hostname '{}'", payload.hostname)));
    }

    // Pastikan catch-all tetap ada di akhir
    let has_catchall = config.ingress.iter().any(|r| r.hostname.is_none());
    if !has_catchall {
        config.ingress.push(IngressRule { hostname: None, service: "http_status:404".to_string(), cname_active: None });
    }

    let new_yaml = build_config_yaml(&config)?;
    write_config_file(&password, &new_yaml)?;

    restart_service_internal(&password)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": format!("Route '{}' deleted and service restarted.", payload.hostname)
    })))
}

/// Restart cloudflared service
pub async fn restart_service(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    restart_service_internal(&password)?;
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "cloudflared service restarted"
    })))
}

/// Daftarkan CNAME DNS ke Cloudflare secara manual
pub async fn register_dns_cname(
    Extension(_auth): Extension<AuthUser>,
    Json(payload): Json<RegisterDnsRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if payload.tunnel_name.is_empty() || payload.hostname.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "tunnel_name and hostname must not be empty".to_string()));
    }

    // Jalankan tanpa sudo agar cert.pem di home directory user terbaca
    let dns_output = Command::new("cloudflared")
        .args(["tunnel", "route", "dns", &payload.tunnel_name, &payload.hostname])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to register DNS: {}", e)))?;

    let dns_stdout = String::from_utf8_lossy(&dns_output.stdout).to_string();
    let dns_stderr = String::from_utf8_lossy(&dns_output.stderr).to_string();
    let dns_combined = format!("{}{}", dns_stdout, dns_stderr);

    if dns_output.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("CNAME registered successfully: {}", dns_combined.trim())
        })))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("DNS Registration failed: {}", dns_combined.trim())))
    }
}

/// Start cloudflared service
pub async fn start_service(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    
    // Coba start secara langsung
    let out = sudo_exec(&password, &["systemctl", "start", "cloudflared"])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to start service: {}", e)))?;

    if out.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "cloudflared service started"
        })))
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        
        // Jika service belum ter-install, coba install otomatis lalu start lagi
        if stderr.contains("not found") || stderr.contains("Failed to start cloudflared.service") {
            let install_out = sudo_exec(&password, &["cloudflared", "service", "install"]);
            if let Ok(install) = install_out {
                if install.status.success() {
                    let retry_out = sudo_exec(&password, &["systemctl", "start", "cloudflared"]);
                    if let Ok(retry) = retry_out {
                        if retry.status.success() {
                            return Ok(Json(serde_json::json!({
                                "status": "success",
                                "message": "cloudflared service installed and started"
                            })));
                        }
                    }
                }
            }
        }
        
        Err((StatusCode::INTERNAL_SERVER_ERROR, stderr))
    }
}

// ─────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────

fn restart_service_internal(password: &str) -> Result<(), (StatusCode, String)> {
    let out = sudo_exec(password, &["systemctl", "restart", "cloudflared"])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to restart: {}", e)))?;

    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        
        // Jika service belum ter-install, otomatis jalankan install service
        if stderr.contains("not found") || stderr.contains("Failed to restart cloudflared.service") {
            let install_out = sudo_exec(password, &["cloudflared", "service", "install"]);
            if let Ok(install) = install_out {
                if install.status.success() {
                    let retry_out = sudo_exec(password, &["systemctl", "restart", "cloudflared"]);
                    if let Ok(retry) = retry_out {
                        if retry.status.success() {
                            return Ok(());
                        }
                    }
                }
            }
        }
        
        Err((StatusCode::INTERNAL_SERVER_ERROR, stderr))
    }
}

/// Baca config.yml via sudo cat (agar bisa baca /etc/cloudflared)
fn read_config_file(password: &str) -> Result<String, (StatusCode, String)> {
    // We cannot reliably use `Path::new().exists()` because the parent dir is root-owned.
    // Instead, just execute `sudo cat` and check the error.
    let out = sudo_exec(password, &["cat", CONFIG_PATH])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to run cat: {}", e)))?;

    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        // If it fails, assume the file does not exist
        Err((StatusCode::NOT_FOUND, format!("Config file not found at {}", CONFIG_PATH)))
    }
}

/// Tulis config.yml
fn write_config_file(password: &str, content: &str) -> Result<(), (StatusCode, String)> {
    // Escape single quotes for bash
    let escaped_content = content.replace("'", "'\\''");
    let cmd = format!("echo '{}' > {}", escaped_content, CONFIG_PATH);

    let out = sudo_exec(password, &["sh", "-c", &cmd])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to apply new config: {}", e)))?;

    if out.status.success() {
        Ok(())
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, String::from_utf8_lossy(&out.stderr).to_string()))
    }
}

/// Parse config.yml menggunakan serde_yaml
fn parse_config(raw: &str) -> Result<LocalTunnelConfig, (StatusCode, String)> {
    serde_yaml::from_str(raw).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse config.yml: {}", e))
    })
}


/// Build kembali YAML string dari struct LocalTunnelConfig menggunakan serde_yaml
fn build_config_yaml(config: &LocalTunnelConfig) -> Result<String, (StatusCode, String)> {
    serde_yaml::to_string(config).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to generate config.yml: {}", e))
    })
}
