use axum::{extract::Extension, Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::process::Command;
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemdService {
    pub unit: String,
    pub load: String,
    pub active: String,
    pub sub: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct ServiceActionReq {
    pub action: String, // start, stop, restart, enable, disable
    pub service_name: String,
}

pub async fn list_services_handler() -> Result<Json<Vec<SystemdService>>, (StatusCode, Json<serde_json::Value>)> {
    let out = tokio::task::spawn_blocking(|| {
        Command::new("systemctl")
            .args(["list-units", "--type=service", "--all", "--no-pager", "--no-legend"])
            .output()
    }).await.unwrap()
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    let mut services = Vec::new();
    let stdout = String::from_utf8_lossy(&out.stdout);

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            let unit = parts[0].to_string();
            let load = parts[1].to_string();
            let active = parts[2].to_string();
            let sub = parts[3].to_string();
            let description = parts[4..].join(" ");
            
            services.push(SystemdService {
                unit,
                load,
                active,
                sub,
                description,
            });
        }
    }

    Ok(Json(services))
}

pub async fn service_action_handler(
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<ServiceActionReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let allowed_actions = vec!["start", "stop", "restart", "enable", "disable"];
    if !allowed_actions.contains(&payload.action.as_str()) {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid action"}))));
    }

    // Hindari command injection
    if !payload.service_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '@' || c == '_') {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid service name"}))));
    }

    let p = auth.0.pwd;
    let action = payload.action.clone();
    let service = payload.service_name.clone();

    let out = tokio::task::spawn_blocking(move || {
        sudo_exec(&p, &["systemctl", &action, &service])
    }).await.unwrap()
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    if out.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Service {} {}ed successfully", payload.service_name, payload.action)
        })))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("systemctl failed: {}", err)}))))
    }
}
