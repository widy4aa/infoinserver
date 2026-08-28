use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::process::Command;

const CONFIG_PATH: &str = "/etc/cloudflared/config.yml";

/// Representasi ingress rule dari config.yml
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IngressRule {
    pub hostname: Option<String>,
    pub service: String,
}

/// Representasi config.yml yang diparsing
#[derive(Serialize, Deserialize, Debug)]
pub struct LocalTunnelConfig {
    pub tunnel: Option<String>,
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

/// Baca dan parse /etc/cloudflared/config.yml
pub async fn get_local_config() -> Result<Json<LocalTunnelConfig>, (StatusCode, String)> {
    let raw = read_config_file()?;
    let config = parse_config(&raw)?;
    Ok(Json(config))
}

/// Tambah route baru:
/// 1) Daftarkan DNS CNAME via `cloudflared tunnel route dns`
/// 2) Update config.yml
/// 3) Restart service
pub async fn add_local_route(
    Json(payload): Json<AddRouteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Validasi input
    if payload.hostname.is_empty() || payload.service.is_empty() || payload.tunnel_name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "hostname, service, and tunnel_name must not be empty".to_string()));
    }

    // 1. Daftarkan DNS CNAME ke Cloudflare
    let dns_output = Command::new("cloudflared")
        .args(["tunnel", "route", "dns", &payload.tunnel_name, &payload.hostname])
        .output()
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
    let raw = read_config_file()?;
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
    };
    // Pisahkan catch-all dan rules biasa
    let mut normal_rules: Vec<IngressRule> = config.ingress.into_iter()
        .filter(|r| r.hostname.is_some())
        .collect();
    normal_rules.push(new_rule);
    // Tambahkan catch-all di akhir
    normal_rules.push(IngressRule { hostname: None, service: "http_status:404".to_string() });
    config.ingress = normal_rules;

    // 4. Tulis kembali ke config.yml
    let new_yaml = build_config_yaml(&config)?;
    write_config_file(&new_yaml)?;

    // 5. Restart service
    let _ = restart_service_internal();

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": format!("Route '{}' -> '{}' added and service restarted.", payload.hostname, payload.service),
        "dns_result": dns_msg
    })))
}

/// Hapus route dari config dan restart service
pub async fn delete_local_route(
    Json(payload): Json<DeleteRouteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if payload.hostname.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "hostname must not be empty".to_string()));
    }

    let raw = read_config_file()?;
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
        config.ingress.push(IngressRule { hostname: None, service: "http_status:404".to_string() });
    }

    let new_yaml = build_config_yaml(&config)?;
    write_config_file(&new_yaml)?;

    let _ = restart_service_internal();

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": format!("Route '{}' deleted and service restarted.", payload.hostname)
    })))
}

/// Restart cloudflared service
pub async fn restart_service() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    restart_service_internal()?;
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "cloudflared service restarted"
    })))
}

// ─────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────

fn restart_service_internal() -> Result<(), (StatusCode, String)> {
    let out = Command::new("sudo")
        .args(["systemctl", "restart", "cloudflared"])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to restart: {}", e)))?;

    if out.status.success() {
        Ok(())
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, String::from_utf8_lossy(&out.stderr).to_string()))
    }
}

/// Baca config.yml via sudo cat (agar bisa baca /etc/cloudflared)
fn read_config_file() -> Result<String, (StatusCode, String)> {
    if !std::path::Path::new(CONFIG_PATH).exists() {
        return Err((StatusCode::NOT_FOUND, format!("Config file not found at {}", CONFIG_PATH)));
    }

    let out = Command::new("sudo")
        .args(["cat", CONFIG_PATH])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read config: {}", e)))?;

    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, String::from_utf8_lossy(&out.stderr).to_string()))
    }
}

/// Tulis config.yml via `sudo tee`
fn write_config_file(content: &str) -> Result<(), (StatusCode, String)> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new("sudo")
        .args(["tee", CONFIG_PATH])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write config: {}", e)))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(content.as_bytes())
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write stdin: {}", e)))?;
    }

    let out = child.wait_with_output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to wait for tee: {}", e)))?;

    if out.status.success() {
        Ok(())
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, String::from_utf8_lossy(&out.stderr).to_string()))
    }
}

/// Parse config.yml secara manual (tanpa serde_yaml dependency yang berat)
/// Format yang didukung sesuai dokumentasi cloudflared
fn parse_config(raw: &str) -> Result<LocalTunnelConfig, (StatusCode, String)> {
    let mut tunnel: Option<String> = None;
    let mut credentials_file: Option<String> = None;
    let mut ingress: Vec<IngressRule> = Vec::new();

    let mut in_ingress = false;
    let mut current_hostname: Option<String> = None;

    for line in raw.lines() {
        let trimmed = line.trim();

        // Skip comment dan baris kosong
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        // Deteksi apakah kita masuk ke block ingress
        if trimmed == "ingress:" {
            in_ingress = true;
            continue;
        }

        if !in_ingress {
            // Parse field top-level
            if let Some(val) = trimmed.strip_prefix("tunnel:") {
                tunnel = Some(val.trim().to_string());
            } else if let Some(val) = trimmed.strip_prefix("credentials-file:") {
                credentials_file = Some(val.trim().to_string());
            }
        } else {
            // Kita di dalam block ingress
            if trimmed.starts_with("- hostname:") {
                // Simpan hostname sementara
                let val = trimmed.trim_start_matches("- hostname:").trim().to_string();
                current_hostname = Some(val);
            } else if trimmed.starts_with("hostname:") {
                let val = trimmed.trim_start_matches("hostname:").trim().to_string();
                current_hostname = Some(val);
            } else if trimmed.starts_with("- service:") {
                // Catch-all rule: - service: http_status:404
                let val = trimmed.trim_start_matches("- service:").trim().to_string();
                ingress.push(IngressRule {
                    hostname: None,
                    service: val,
                });
                current_hostname = None;
            } else if trimmed.starts_with("service:") {
                let val = trimmed.trim_start_matches("service:").trim().to_string();
                ingress.push(IngressRule {
                    hostname: current_hostname.take(),
                    service: val,
                });
            }
        }
    }

    Ok(LocalTunnelConfig {
        tunnel,
        credentials_file,
        ingress,
    })
}

/// Build kembali YAML string dari struct LocalTunnelConfig
fn build_config_yaml(config: &LocalTunnelConfig) -> Result<String, (StatusCode, String)> {
    let mut yaml = String::new();

    if let Some(ref t) = config.tunnel {
        yaml.push_str(&format!("tunnel: {}\n", t));
    }
    if let Some(ref cf) = config.credentials_file {
        yaml.push_str(&format!("credentials-file: {}\n", cf));
    }
    yaml.push('\n');
    yaml.push_str("ingress:\n");

    for rule in &config.ingress {
        if let Some(ref hostname) = rule.hostname {
            yaml.push_str(&format!("  - hostname: {}\n", hostname));
            yaml.push_str(&format!("    service: {}\n", rule.service));
        } else {
            // Catch-all rule
            yaml.push_str(&format!("  - service: {}\n", rule.service));
        }
    }

    Ok(yaml)
}
