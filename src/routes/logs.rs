use axum::{extract::{State, Extension}, Json, http::StatusCode};
use serde::Serialize;
use sqlx::FromRow;
use crate::AppState;
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;

#[derive(Serialize, FromRow)]
pub struct ActivityLog {
    pub id: i64,
    pub timestamp: String,
    pub level: String,
    pub action: String,
    pub detail: Option<String>,
}

/// Utility function untuk insert log dari route mana saja
pub async fn log_activity(pool: &sqlx::SqlitePool, level: &str, action: &str, detail: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query("INSERT INTO activity_log (timestamp, level, action, detail) VALUES (?, ?, ?, ?)")
        .bind(now)
        .bind(level)
        .bind(action)
        .bind(detail)
        .execute(pool)
        .await;
}

pub async fn get_activity_logs_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<ActivityLog>>, (StatusCode, String)> {
    let logs = sqlx::query_as::<_, ActivityLog>(
        "SELECT id, timestamp, level, action, detail 
         FROM activity_log 
         ORDER BY id DESC LIMIT 200"
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(logs))
}

pub async fn get_bash_history_handler(
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    
    // Coba baca .bash_history dari root dan user home
    // Gunakan sudo cat untuk memastikan permission
    let mut history_lines = Vec::new();

    // 1. Baca history root
    if let Ok(out) = sudo_exec(&password, &["cat", "/root/.bash_history"]) {
        if out.status.success() {
            let root_hist = String::from_utf8_lossy(&out.stdout).to_string();
            for line in root_hist.lines() {
                if !line.trim().is_empty() {
                    history_lines.push(format!("[root] {}", line.trim()));
                }
            }
        }
    }

    // 2. Baca history user saat ini (login user)
    let home = std::env::var("HOME").unwrap_or_else(|_| "".to_string());
    if !home.is_empty() && home != "/root" {
        let user_hist_path = format!("{}/.bash_history", home);
        if let Ok(out) = sudo_exec(&password, &["cat", &user_hist_path]) {
            if out.status.success() {
                let user_name = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
                let user_hist = String::from_utf8_lossy(&out.stdout).to_string();
                for line in user_hist.lines() {
                    if !line.trim().is_empty() {
                        history_lines.push(format!("[{}] {}", user_name, line.trim()));
                    }
                }
            }
        }
    }

    // Ambil 500 baris terakhir agar tidak membebani frontend
    let max_lines = 500;
    let total_len = history_lines.len();
    if total_len > max_lines {
        history_lines = history_lines.into_iter().skip(total_len - max_lines).collect();
    }

    Ok(Json(history_lines))
}
