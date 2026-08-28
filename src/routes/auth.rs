// src/routes/auth.rs
// POST /api/auth/login — verifikasi password Linux user via PAM, return JWT

use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use pam::Client;
use crate::auth::jwt::create_token;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
}

pub async fn login_handler(
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    let username = payload.username.trim().to_string();
    let password = payload.password.clone();

    if username.is_empty() || password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Username and password required" })),
        ));
    }

    // Verifikasi password via PAM (Linux system auth) — jalankan di blocking thread
    let auth_result = tokio::task::spawn_blocking({
        let username = username.clone();
        let password = password.clone();
        move || -> bool {
            // Coba service "system-auth" dulu, fallback ke "login"
            let client = Client::with_password("system-auth")
                .or_else(|_| Client::with_password("login"));

            let mut client = match client {
                Ok(c) => c,
                Err(_) => return false,
            };

            client.conversation_mut().set_credentials(&username, &password);
            client.authenticate().is_ok()
        }
    })
    .await
    .unwrap_or(false);

    if !auth_result {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid username or password" })),
        ));
    }

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "changeme-jwt-secret".to_string());
    match create_token(&username, &password, &secret) {
        Ok(token) => Ok(Json(LoginResponse { token, username })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Token generation failed: {}", e) })),
        )),
    }
}
