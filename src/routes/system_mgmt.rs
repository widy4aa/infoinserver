use axum::{Json, http::StatusCode};
use serde::Serialize;
use tokio::process::Command;
use std::time::Duration;

#[derive(Serialize)]
pub struct MgmtResponse {
    pub status: String,
    pub message: String,
}

pub async fn update_dashboard_handler() -> Result<Json<MgmtResponse>, (StatusCode, String)> {
    // 1. Git Pull
    let pull_cmd = Command::new("git")
        .arg("pull")
        .output()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to run git pull: {}", e)))?;

    if !pull_cmd.status.success() {
        let stderr = String::from_utf8_lossy(&pull_cmd.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Git pull failed: {}", stderr)));
    }

    // 2. Cargo Build --release
    let build_cmd = Command::new("cargo")
        .args(["build", "--release"])
        .output()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to run cargo build: {}", e)))?;

    if !build_cmd.status.success() {
        let stderr = String::from_utf8_lossy(&build_cmd.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Build failed: {}", stderr)));
    }

    // 3. Restart process in background jika build sukses
    tokio::spawn(async {
        // Beri waktu agar respons HTTP berhasil dikirim ke frontend terlebih dahulu
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Memanggil skrip restart terpisah
        let _ = Command::new("bash")
            .arg("-c")
            .arg("./stop.sh && ./start.sh")
            .spawn();
        
        // Matikan instance saat ini
        std::process::exit(0);
    });

    Ok(Json(MgmtResponse {
        status: "success".to_string(),
        message: "Update berhasil di-build. Dashboard sedang di-restart, silakan refresh halaman dalam 5 detik.".to_string(),
    }))
}

pub async fn reboot_server_handler() -> Result<Json<MgmtResponse>, (StatusCode, String)> {
    // Mengeksekusi perintah reboot OS
    let reboot_cmd = Command::new("sudo")
        .arg("reboot")
        .output()
        .await;

    // Fallback jika tanpa sudo bisa (biasanya di environment root)
    let final_status = match reboot_cmd {
        Ok(out) if out.status.success() => Ok(out),
        _ => {
            Command::new("reboot").output().await
        }
    };

    match final_status {
        Ok(out) if out.status.success() => {
            Ok(Json(MgmtResponse {
                status: "success".to_string(),
                message: "Server is rebooting now...".to_string(),
            }))
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Reboot failed (permission denied?): {}", stderr)))
        }
        Err(e) => {
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Reboot execution failed: {}", e)))
        }
    }
}
