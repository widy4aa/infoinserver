use axum::{extract::State, Json, http::StatusCode};
use serde::Serialize;
use sqlx::FromRow;
use crate::AppState;

#[derive(Serialize, FromRow)]
pub struct MetricsHistoryPoint {
    pub timestamp: String,
    pub cpu_usage: f64,
    pub mem_used_bytes: i64,
    pub mem_total_bytes: i64,
    pub disk_used_bytes: i64,
    pub disk_total_bytes: i64,
    pub net_rx_bytes: Option<i64>,
    pub net_tx_bytes: Option<i64>,
}

pub async fn get_metrics_history_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<MetricsHistoryPoint>>, (StatusCode, String)> {
    let history = sqlx::query_as::<_, MetricsHistoryPoint>(
        "SELECT timestamp, cpu_usage, mem_used_bytes, mem_total_bytes, disk_used_bytes, disk_total_bytes, net_rx_bytes, net_tx_bytes 
         FROM system_metrics_history 
         ORDER BY id ASC LIMIT 288" // 24 jam x 12 (karena per 5 menit)
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(history))
}
