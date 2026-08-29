use axum::{extract::{Path, Query, State}, Json, http::StatusCode};
use serde::Deserialize;
use crate::services::compose_manager::{
    deploy_compose, stop_compose, restart_compose, rebuild_compose,
    scale_service, compose_logs, compose_ps, list_compose_projects,
    get_compose_yaml, update_compose_yaml, delete_compose_project,
};
use crate::ContainerState;

// ─────────────────────────────────────────────────────────────────────────────
// Request types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DeployRequest {
    pub name: String,
    pub yaml: String,
}

#[derive(Deserialize)]
pub struct ScaleRequest {
    pub service: String,
    pub count: u32,
}

#[derive(Deserialize)]
pub struct UpdateYamlRequest {
    pub yaml: String,
}

#[derive(Deserialize)]
pub struct LogsQuery {
    pub service: Option<String>,
    pub tail: Option<u32>,
}

#[derive(Deserialize)]
pub struct DeleteQuery {
    pub remove_volumes: Option<bool>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/compose/projects
pub async fn list_projects_handler(
    State(state): State<ContainerState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    match list_compose_projects(rt) {
        Ok(projects) => Ok(Json(serde_json::json!(projects))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// POST /api/compose/deploy
pub async fn deploy_project_handler(
    State(state): State<ContainerState>,
    Json(payload): Json<DeployRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    match deploy_compose(rt, &payload.name, &payload.yaml) {
        Ok(out) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Project '{}' deployed.", payload.name),
            "output": out
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// POST /api/compose/{name}/stop
pub async fn stop_project_handler(
    State(state): State<ContainerState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    match stop_compose(rt, &name) {
        Ok(msg) => Ok(Json(serde_json::json!({ "status": "success", "message": msg }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// POST /api/compose/{name}/restart
pub async fn restart_project_handler(
    State(state): State<ContainerState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    match restart_compose(rt, &name) {
        Ok(msg) => Ok(Json(serde_json::json!({ "status": "success", "message": msg }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// POST /api/compose/{name}/rebuild
pub async fn rebuild_project_handler(
    State(state): State<ContainerState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    match rebuild_compose(rt, &name) {
        Ok(out) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Project '{}' rebuilt.", name),
            "output": out
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// GET /api/compose/{name}/ps — status per-service
pub async fn project_services_handler(
    State(state): State<ContainerState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    match compose_ps(rt, &name) {
        Ok(services) => Ok(Json(serde_json::json!(services))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// GET /api/compose/{name}/logs?service=xxx&tail=100
pub async fn project_logs_handler(
    State(state): State<ContainerState>,
    Path(name): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    let tail = q.tail.unwrap_or(150);
    let service = q.service.as_deref();

    match compose_logs(rt, &name, service, tail) {
        Ok(logs) => Ok(Json(serde_json::json!({ "status": "success", "logs": logs }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// POST /api/compose/{name}/scale
pub async fn scale_service_handler(
    State(state): State<ContainerState>,
    Path(name): Path<String>,
    Json(payload): Json<ScaleRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    match scale_service(rt, &name, &payload.service, payload.count) {
        Ok(out) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Scaled '{}' in project '{}' to {} replica(s).", payload.service, name, payload.count),
            "output": out
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// GET /api/compose/{name}/yaml
pub async fn get_yaml_handler(
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match get_compose_yaml(&name) {
        Ok(yaml) => Ok(Json(serde_json::json!({ "yaml": yaml }))),
        Err(e) => Err((StatusCode::NOT_FOUND, e)),
    }
}

/// PUT /api/compose/{name}/yaml — update YAML + rebuild
pub async fn update_yaml_handler(
    State(state): State<ContainerState>,
    Path(name): Path<String>,
    Json(payload): Json<UpdateYamlRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    match update_compose_yaml(rt, &name, &payload.yaml) {
        Ok(out) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Project '{}' YAML updated and redeployed.", name),
            "output": out
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// DELETE /api/compose/{name}?remove_volumes=true
pub async fn delete_project_handler(
    State(state): State<ContainerState>,
    Path(name): Path<String>,
    Query(q): Query<DeleteQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rt = state.runtime.read().await;
    let rt = rt.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "No container runtime available.".to_string())
    })?;

    let remove_volumes = q.remove_volumes.unwrap_or(false);
    match delete_compose_project(rt, &name, remove_volumes) {
        Ok(msg) => Ok(Json(serde_json::json!({ "status": "success", "message": msg }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}
