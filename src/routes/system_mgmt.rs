use axum::{Json, http::StatusCode, extract::Extension};
use serde::Serialize;
use tokio::process::Command;
use std::time::Duration;
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;

#[derive(Serialize)]
pub struct MgmtResponse {
    pub status: String,
    pub message: String,
}

pub async fn update_dashboard_handler(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Result<Json<MgmtResponse>, (StatusCode, String)> {
    let pull_cmd = Command::new("git")
        .arg("pull")
        .output()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to run git pull: {}", e)))?;

    if !pull_cmd.status.success() {
        let stderr = String::from_utf8_lossy(&pull_cmd.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Git pull failed: {}", stderr)));
    }

    let build_cmd = Command::new("cargo")
        .args(["build", "--release"])
        .output()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to run cargo build: {}", e)))?;

    if !build_cmd.status.success() {
        let stderr = String::from_utf8_lossy(&build_cmd.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Build failed: {}", stderr)));
    }

    tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = Command::new("bash")
            .arg("-c")
            .arg("./stop.sh && ./start.sh")
            .spawn();
        std::process::exit(0);
    });

    crate::routes::logs::log_activity(&state.db_pool, "WARNING", "System Update", "User initiated a dashboard update and rebuild").await;

    Ok(Json(MgmtResponse {
        status: "success".to_string(),
        message: "Update berhasil di-build. Dashboard sedang di-restart, silakan refresh halaman dalam 5 detik.".to_string(),
    }))
}

pub async fn reboot_server_handler(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<MgmtResponse>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    // Jalankan di blocking thread karena sudo_exec blocking
    let result = tokio::task::spawn_blocking(move || {
        sudo_exec(&password, &["reboot"])
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Reboot failed: {}", e)))?;

    if result.status.success() {
        crate::routes::logs::log_activity(&state.db_pool, "CRITICAL", "System Reboot", "Server is rebooting").await;
        Ok(Json(MgmtResponse {
            status: "success".to_string(),
            message: "Server is rebooting now...".to_string(),
        }))
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Reboot failed: {}", stderr)))
    }
}
