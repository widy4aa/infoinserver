use axum::{extract::State, Json, http::StatusCode};
use serde::Serialize;
use sqlx::FromRow;
use crate::AppState;

#[derive(Serialize, FromRow)]
pub struct ActivityLog {
    pub id: i64,
    pub timestamp: String,
    pub level: String,
    pub action: String,
    pub detail: Option<String>,
}

pub async fn get_activity_logs_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<ActivityLog>>, (StatusCode, String)> {
    let logs = sqlx::query_as::<_, ActivityLog>(
        "SELECT id, timestamp, level, action, detail 
         FROM activity_log 
         ORDER BY id DESC LIMIT 100"
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(logs))
}
