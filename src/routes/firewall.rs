use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Debug)]
pub struct UfwStatus {
    pub enabled: bool,
    pub rules: Vec<String>,
}

#[derive(Deserialize)]
pub struct UfwActionRequest {
    // "allow", "deny", or "delete"
    pub action: String,
    // e.g. "80", "8080/tcp", "22"
    pub port: String,
}

pub async fn get_ufw_status_handler() -> Result<Json<UfwStatus>, (StatusCode, String)> {
    let output = Command::new("sudo")
        .arg("ufw")
        .arg("status")
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to run UFW: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Output ufw status biasanya: "Status: active" diikuti baris rules, atau "Status: inactive"
    let enabled = stdout.contains("Status: active");
    
    let mut rules = Vec::new();
    if enabled {
        for line in stdout.lines().skip(4) { // Skip headers
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                rules.push(trimmed.to_string());
            }
        }
    }

    Ok(Json(UfwStatus { enabled, rules }))
}

pub async fn toggle_ufw_handler() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Mengecek status dulu
    let is_active = Command::new("sudo")
        .args(["ufw", "status"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).contains("Status: active"))
        .unwrap_or(false);

    let action = if is_active { "disable" } else { "enable" };
    
    let output = Command::new("sudo")
        .args(["ufw", "--force", action])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to toggle UFW: {}", e)))?;

    if output.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("UFW has been {}d", action)
        })))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to execute toggle command".to_string()))
    }
}

pub async fn manage_ufw_rule_handler(Json(payload): Json<UfwActionRequest>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !["allow", "deny", "delete"].contains(&payload.action.as_str()) {
        return Err((StatusCode::BAD_REQUEST, "Invalid action".to_string()));
    }

    // Validasi basic untuk string port (hindari command injection)
    if !payload.port.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') {
        return Err((StatusCode::BAD_REQUEST, "Invalid port format".to_string()));
    }

    let mut args = vec!["ufw"];
    
    if payload.action == "delete" {
        // e.g. sudo ufw delete allow 80
        // Untuk amannya, kita delete allow rule saja dalam konteks sederhana, atau pass 2 parameter tambahan via API nanti.
        // Di sini saya implementasi sederhana
        return Err((StatusCode::NOT_IMPLEMENTED, "Delete specific rule is complex due to structure, use CLI for advanced management".to_string()));
    } else {
        args.push(&payload.action);
        args.push(&payload.port);
    }

    let output = Command::new("sudo")
        .args(&args)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to apply UFW rule: {}", e)))?;

    if output.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("UFW rule {} {} applied", payload.action, payload.port)
        })))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to apply rule: {}", err)))
    }
}