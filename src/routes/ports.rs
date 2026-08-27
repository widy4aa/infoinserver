use axum::{extract::{State, Path, Json as AxumJson}, Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use crate::services::port_scanner::{get_listening_ports, PortInfo};
use crate::services::nmap_scanner::run_nmap_scan;
use crate::AppState;
use std::net::IpAddr;
use sqlx::FromRow;

pub async fn listening_ports_handler() -> Json<Vec<PortInfo>> {
    let ports = get_listening_ports();
    Json(ports)
}

#[derive(Deserialize)]
pub struct ScanRequest {
    pub target: String,
}

#[derive(Serialize)]
pub struct ScanResponse {
    pub job_id: i64,
    pub status: String,
}

pub async fn trigger_scan_handler(
    State(state): State<AppState>,
    AxumJson(payload): AxumJson<ScanRequest>,
) -> Result<Json<ScanResponse>, (StatusCode, String)> {
    // Validasi input: pastikan target adalah IP atau hostname valid untuk cegah command injection
    let is_valid = payload.target == "localhost" || payload.target.parse::<IpAddr>().is_ok();
    
    if !is_valid {
        return Err((StatusCode::BAD_REQUEST, "Invalid target. Use 'localhost' or a valid IP address.".to_string()));
    }

    // Insert pending job to DB
    let result = sqlx::query(
        "INSERT INTO port_scan_jobs (target, status) VALUES (?, 'pending')"
    )
    .bind(payload.target.clone())
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let job_id = result.last_insert_rowid();

    // Spawn background task
    let db_pool = state.db_pool.clone();
    let target = payload.target.clone();
    tokio::spawn(async move {
        run_nmap_scan(db_pool, job_id, target).await;
    });

    Ok(Json(ScanResponse {
        job_id,
        status: "Scan initiated".to_string(),
    }))
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct ScanJobRow {
    pub id: i64,
    pub target: String,
    pub status: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub result_json: Option<String>,
}

pub async fn get_scan_status_handler(
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
) -> Result<Json<ScanJobRow>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, ScanJobRow>(
        "SELECT id, target, status, started_at, finished_at, result_json FROM port_scan_jobs WHERE id = ?"
    )
    .bind(job_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match row {
        Some(job) => Ok(Json(job)),
        None => Err((StatusCode::NOT_FOUND, "Job not found".to_string())),
    }
}
