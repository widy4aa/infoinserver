use axum::{extract::Extension, Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::io::Write;
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;

#[derive(Serialize)]
pub struct CronResponse {
    pub crontab: String,
}

#[derive(Deserialize)]
pub struct UpdateCronReq {
    pub crontab: String,
}

pub async fn get_cron_handler(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<CronResponse>, (StatusCode, Json<serde_json::Value>)> {
    let p = auth.0.pwd;
    
    let out = tokio::task::spawn_blocking(move || {
        sudo_exec(&p, &["crontab", "-l", "-u", "root"])
    }).await.unwrap()
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    // Kalau success, kembalikan isinya. 
    // Kalau fail (misal "no crontab for root"), itu wajar, kita kembalikan string kosong.
    if out.status.success() {
        Ok(Json(CronResponse { crontab: String::from_utf8_lossy(&out.stdout).to_string() }))
    } else {
        Ok(Json(CronResponse { crontab: "".to_string() }))
    }
}

pub async fn update_cron_handler(
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<UpdateCronReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let p = auth.0.pwd;
    let content = payload.crontab.clone() + "\n"; // pastikan ada newline di akhir
    
    // Tulis ke temp file
    let tmp_path = "/tmp/infoinserver_cron.txt";
    if let Err(e) = std::fs::write(tmp_path, content) {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to write tmp file: {}", e)}))));
    }

    let out = tokio::task::spawn_blocking(move || {
        let res = sudo_exec(&p, &["crontab", "-u", "root", tmp_path]);
        // Bersihkan temp file
        let _ = std::fs::remove_file(tmp_path);
        res
    }).await.unwrap()
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    if out.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Root crontab updated successfully"
        })))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("crontab failed: {}", err)}))))
    }
}
