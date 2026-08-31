use axum::{Json, http::StatusCode, extract::Extension};
use serde::{Deserialize, Serialize};
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;

#[derive(Serialize, Deserialize, Debug)]
pub struct UfwStatus {
    pub enabled: bool,
    pub rules: Vec<String>,
}

#[derive(Deserialize)]
pub struct UfwActionRequest {
    pub action: String,
    pub port: String,
}

pub async fn get_ufw_status_handler(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<UfwStatus>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    let output = tokio::task::spawn_blocking(move || {
        sudo_exec(&password, &["ufw", "status"])
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to run UFW: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let enabled = stdout.contains("Status: active");

    let mut rules = Vec::new();
    if enabled {
        for line in stdout.lines().skip(4) {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                rules.push(trimmed.to_string());
            }
        }
    }

    Ok(Json(UfwStatus { enabled, rules }))
}

pub async fn toggle_ufw_handler(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    let password2 = password.clone();

    // Cek status dulu
    let status_out = tokio::task::spawn_blocking(move || {
        sudo_exec(&password, &["ufw", "status"])
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let is_active = String::from_utf8_lossy(&status_out.stdout).contains("Status: active");
    let action = if is_active { "disable" } else { "enable" };

    let output = tokio::task::spawn_blocking(move || {
        sudo_exec(&password2, &["ufw", "--force", action])
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to toggle UFW: {}", e)))?;

    if output.status.success() {
        crate::routes::logs::log_activity(&state.db_pool, "WARNING", "Firewall Toggle", &format!("UFW state changed to: {}", action)).await;
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("UFW has been {}d", action)
        })))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to execute toggle command".to_string()))
    }
}

pub async fn manage_ufw_rule_handler(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<UfwActionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !["allow", "deny", "delete"].contains(&payload.action.as_str()) {
        return Err((StatusCode::BAD_REQUEST, "Invalid action".to_string()));
    }

    if !payload.port.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') {
        return Err((StatusCode::BAD_REQUEST, "Invalid port format".to_string()));
    }

    if payload.action == "delete" {
        return Err((StatusCode::NOT_IMPLEMENTED, "Use CLI for advanced rule deletion".to_string()));
    }

    let password = auth.0.pwd.clone();
    let action = payload.action.clone();
    let port = payload.port.clone();

    let output = tokio::task::spawn_blocking(move || {
        sudo_exec(&password, &["ufw", &action, &port])
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to apply UFW rule: {}", e)))?;

    if output.status.success() {
        crate::routes::logs::log_activity(&state.db_pool, "WARNING", "Firewall Rule", &format!("Rule '{}' applied on port {}", payload.action, payload.port)).await;
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("UFW rule {} {} applied", payload.action, payload.port)
        })))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to apply rule: {}", err)))
    }
}
