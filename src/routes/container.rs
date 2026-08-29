use axum::{extract::{Path, Query, State}, Json, http::StatusCode};
use serde::Deserialize;
use crate::services::container_runtime::{
    detect_runtime, list_containers, container_action,
    create_container, inspect_container, container_logs,
};
use crate::ContainerState;

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateContainerRequest {
    pub name: String,
    pub image: String,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub env_vars: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<String>,
    pub restart_policy: Option<String>,
}

#[derive(Deserialize)]
pub struct LogsQuery {
    pub tail: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/container/runtime — info runtime yang terdeteksi
pub async fn get_runtime_info_handler(
    State(state): State<ContainerState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    match &*rt {
        Some(r) => Ok(Json(serde_json::json!({
            "available": true,
            "kind": format!("{:?}", r.kind),
            "binary": r.binary,
            "version": r.version,
            "compose_binary": r.compose_binary,
            "compose_available": !r.compose_binary.is_empty(),
        }))),
        None => Ok(Json(serde_json::json!({
            "available": false,
            "kind": null,
            "binary": null,
            "version": null,
            "compose_binary": null,
            "compose_available": false,
        }))),
    }
}

/// POST /api/container/runtime/refresh — re-detect runtime
pub async fn refresh_runtime_handler(
    State(state): State<ContainerState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match detect_runtime() {
        Ok(rt) => {
            let kind = format!("{:?}", rt.kind);
            let version = rt.version.clone();
            let binary = rt.binary.clone();
            *state.runtime.write().await = Some(rt);
            Ok(Json(serde_json::json!({
                "status": "success",
                "message": format!("Detected {} v{}", binary, version),
                "kind": kind,
            })))
        }
        Err(e) => {
            *state.runtime.write().await = None;
            Err((StatusCode::SERVICE_UNAVAILABLE, e))
        }
    }
}

/// GET /api/container/list — list semua container
pub async fn list_containers_handler(
    State(state): State<ContainerState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available. Install podman or docker.".to_string())
    })?;

    match list_containers(rt) {
        Ok(containers) => Ok(Json(serde_json::json!(containers))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// POST /api/container/{action}/{id}
pub async fn container_action_handler(
    State(state): State<ContainerState>,
    Path((action, id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    match container_action(rt, &action, &id) {
        Ok(msg) => Ok(Json(serde_json::json!({ "status": "success", "message": msg }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}

/// POST /api/container/create
pub async fn create_container_handler(
    State(state): State<ContainerState>,
    Json(payload): Json<CreateContainerRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    let restart = payload.restart_policy.as_deref();

    match create_container(rt, &payload.name, &payload.image, &payload.ports, &payload.env_vars, &payload.volumes, restart) {
        Ok(id) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Container '{}' created", payload.name),
            "container_id": id
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// GET /api/container/inspect/{id}
pub async fn inspect_handler(
    State(state): State<ContainerState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    match inspect_container(rt, &id) {
        Ok(data) => Ok(Json(data)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// GET /api/container/logs/{id}?tail=100
pub async fn logs_handler(
    State(state): State<ContainerState>,
    Path(id): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    let tail = q.tail.unwrap_or(150);
    match container_logs(rt, &id, tail) {
        Ok(logs) => Ok(Json(serde_json::json!({ "status": "success", "logs": logs }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}
