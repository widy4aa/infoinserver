use axum::{extract::State, Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use crate::AppState;
use std::process::Command;

#[derive(Deserialize)]
pub struct CreateContainerRequest {
    pub name: String,
    pub image: String,
    // Port mappings, e.g., ["8080:80"]
    pub ports: Vec<String>,
}

#[derive(Serialize)]
pub struct CreateContainerResponse {
    pub status: String,
    pub message: String,
}

fn is_valid_name_or_image(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':' || c == '/')
}

fn is_valid_port_mapping(s: &str) -> bool {
    // Basic check format "1234:80"
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 { return false; }
    parts[0].parse::<u16>().is_ok() && parts[1].parse::<u16>().is_ok()
}

pub async fn create_container_handler(
    State(_state): State<AppState>,
    Json(payload): Json<CreateContainerRequest>,
) -> Result<Json<CreateContainerResponse>, (StatusCode, String)> {
    
    // 1. Validation
    if !is_valid_name_or_image(&payload.name) {
        return Err((StatusCode::BAD_REQUEST, "Invalid container name".to_string()));
    }
    if !is_valid_name_or_image(&payload.image) {
        return Err((StatusCode::BAD_REQUEST, "Invalid image name".to_string()));
    }
    for p in &payload.ports {
        if !is_valid_port_mapping(p) {
            return Err((StatusCode::BAD_REQUEST, format!("Invalid port mapping format: {}", p)));
        }
    }

    // 2. Build Arguments securely
    let mut args = vec!["run", "-d", "--name", &payload.name];
    
    for p in &payload.ports {
        args.push("-p");
        args.push(p);
    }
    
    args.push(&payload.image);

    // 3. Execute
    let output = Command::new("podman")
        .args(&args)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to spawn podman: {}", e)))?;

    if output.status.success() {
        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Json(CreateContainerResponse {
            status: "success".to_string(),
            message: format!("Container started with ID: {}", container_id)
        }))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Podman error: {}", err)))
    }
}
