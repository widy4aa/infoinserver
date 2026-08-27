use axum::{extract::Path, Json, http::StatusCode};
use serde_json::{json, Value};
use crate::services::podman_cli::{list_containers, perform_container_action};

pub async fn list_containers_handler() -> Result<Json<Value>, (StatusCode, String)> {
    match list_containers() {
        Ok(containers) => Ok(Json(json!(containers))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

pub async fn container_action_handler(
    Path((action, id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, String)> {
    match perform_container_action(&action, &id) {
        Ok(msg) => Ok(Json(json!({ "status": "success", "message": msg }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}