use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use crate::services::speedtest_cli::run_speedtest;
use crate::AppState;

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct SpeedtestHistoryRow {
    pub id: i64,
    pub tested_at: String,
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
    pub server_name: Option<String>,
}

pub async fn get_history_handler(State(state): State<AppState>) -> Result<Json<Vec<SpeedtestHistoryRow>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, SpeedtestHistoryRow>(
        "SELECT id, tested_at, download_mbps, upload_mbps, ping_ms, server_name FROM speedtest_history ORDER BY id DESC LIMIT 20"
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    Ok(Json(rows))
}

pub async fn run_speedtest_handler(State(state): State<AppState>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match run_speedtest(&state.db_pool).await {
        Ok(result) => Ok(Json(serde_json::json!({
            "status": "success",
            "data": result
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}
