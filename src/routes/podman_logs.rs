use axum::{extract::Path, Json, http::StatusCode};
use std::process::Command;

pub async fn get_container_logs_handler(Path(id): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err((StatusCode::BAD_REQUEST, "Invalid container ID".to_string()));
    }

    // Eksekusi podman logs (ambil 100 baris terakhir)
    let output = Command::new("podman")
        .args(["logs", "--tail", "100", &id])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read logs: {}", e)))?;

    // Podman (dan Docker) umumnya mengirimkan log stdout dan stderr
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Gabungkan keduanya
    let mut combined_logs = String::new();
    if !stdout.is_empty() {
        combined_logs.push_str(&stdout);
    }
    if !stderr.is_empty() {
        combined_logs.push_str(&stderr);
    }

    if combined_logs.is_empty() {
        combined_logs = "No logs available or container has no output.".to_string();
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "logs": combined_logs
    })))
}