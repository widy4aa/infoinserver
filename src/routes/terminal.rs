use axum::{Json, http::StatusCode};
use serde_json::Value;
use std::process::Command;

pub async fn start_shellinabox_handler() -> Result<Json<Value>, (StatusCode, String)> {
    // Mengecek apakah shellinaboxd terinstal
    let check_cmd = Command::new("which")
        .arg("shellinaboxd")
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to check for shellinaboxd: {}", e)))?;

    if !check_cmd.status.success() {
        return Err((StatusCode::NOT_IMPLEMENTED, "shellinaboxd is not installed on this server. Please install it first (e.g. sudo apt install shellinabox or paru -S shellinabox).".to_string()));
    }

    // Mengecek apakah sudah berjalan
    let is_running = Command::new("pgrep")
        .arg("-f")
        .arg("shellinaboxd")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    if is_running {
        return Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Shellinabox is already running on port 4200"
        })));
    }

    // Menjalankan Shellinaboxd di background tanpa SSL (--disable-ssl) di port 4200
    // Menggunakan opsi standar untuk testing. Jika ingin SSL harus konfigurasi cert.
    let spawn_result = Command::new("shellinaboxd")
        .args(["-t", "-b", "-p", "4200"])
        .spawn();

    match spawn_result {
        Ok(_) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Shellinabox started successfully on port 4200"
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to start shellinaboxd: {}", e)))
    }
}
