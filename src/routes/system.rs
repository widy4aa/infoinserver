use axum::Json;
use crate::services::proc_reader::{get_system_metrics, SystemMetrics};

pub async fn system_info_handler() -> Json<SystemMetrics> {
    Json(get_system_metrics())
}
