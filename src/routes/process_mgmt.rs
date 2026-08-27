use axum::{extract::{Path, State}, Json, http::StatusCode};
use std::process::Command;
use crate::AppState;
use crate::services::process_info::{get_top_processes, ProcessInfo};

pub async fn list_processes_handler(State(state): State<AppState>) -> Json<Vec<ProcessInfo>> {
    let mut sys_lock = state.sys.lock().unwrap();
    let procs = get_top_processes(&mut sys_lock);
    Json(procs)
}

pub async fn kill_process_handler(Path(pid): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !pid.chars().all(|c| c.is_ascii_digit()) {
        return Err((StatusCode::BAD_REQUEST, "Invalid PID".to_string()));
    }

    let output = Command::new("sudo")
        .args(["kill", "-9", &pid])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to execute kill: {}", e)))?;

    if output.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Process {} killed successfully", pid)
        })))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Kill failed: {}", err)))
    }
}