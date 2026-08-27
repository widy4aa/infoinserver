use axum::{extract::Path, Json, http::StatusCode};
use serde_json::Value;
use std::process::Command;

pub async fn get_container_details_handler(Path(id): Path<String>) -> Result<Json<Value>, (StatusCode, String)> {
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err((StatusCode::BAD_REQUEST, "Invalid container ID".to_string()));
    }

    let output = Command::new("podman")
        .args(["inspect", &id])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to inspect container: {}", e)))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Podman error: {}", err)));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    
    // podman inspect mereturn array [ {..} ]
    let json_val: Value = serde_json::from_str(&json_str)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse JSON: {}", e)))?;

    Ok(Json(json_val))
}
