use axum::{extract::{Path, State, Extension}, Json, http::StatusCode};
use std::process::{Command, Stdio};
use std::io::Write;
use crate::AppState;
use crate::services::process_info::{get_top_processes, ProcessInfo};
use crate::auth::jwt_middleware::AuthUser;

pub async fn list_processes_handler(State(state): State<AppState>) -> Json<Vec<ProcessInfo>> {
    let mut sys_lock = state.sys.lock().unwrap();
    let procs = get_top_processes(&mut sys_lock);
    Json(procs)
}


pub async fn kill_process_handler(
    Path(pid): Path<String>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !pid.chars().all(|c| c.is_ascii_digit()) {
        return Err((StatusCode::BAD_REQUEST, "Invalid PID".to_string()));
    }

    let password = auth.0.pwd.clone();
    let output = sudo_exec(&password, &["kill", "-9", &pid])
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

/// Helper: jalankan command dengan sudo menggunakan password dari session
pub fn sudo_exec(password: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut child = Command::new("sudo")
        .arg("-S")   // baca password dari stdin
        .arg("-p")   // custom prompt (kosong agar tidak bingung output)
        .arg("")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(format!("{}\n", password).as_bytes());
    }

    child.wait_with_output()
}
