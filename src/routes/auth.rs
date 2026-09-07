// src/routes/auth.rs
// POST /api/auth/login    — verifikasi password Linux user via PAM, return JWT
// POST /api/auth/refresh  — perbarui JWT yang masih valid, return JWT baru
//
// GitHub OAuth dihandle oleh Bun+Hono server (frontend-vue/server.ts)

use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use pam::Client;
use crate::auth::jwt::{create_token, verify_token};

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

/// Cek apakah user terdaftar di grup sudo atau wheel
fn is_sudo_user(username: &str) -> bool {
    let output = std::process::Command::new("groups")
        .arg(username)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let groups = String::from_utf8_lossy(&out.stdout);
            groups.contains("sudo") || groups.contains("wheel")
        }
        _ => false,
    }
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

    // Tolak login sebagai root — gunakan user non-root dengan sudo
    if username == "root" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Login as root is not allowed. Use a non-root user with sudo privileges." })),
        ));
    }

    // Verifikasi password via PAM (Linux system auth) — jalankan di blocking thread
    let auth_result = tokio::task::spawn_blocking({
        let username = username.clone();
        let password = password.clone();
        move || -> bool {
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

    let username_clone = username.clone();
    let has_sudo = tokio::task::spawn_blocking(move || is_sudo_user(&username_clone))
        .await
        .unwrap_or(false);

    if !has_sudo {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Access denied. Only users with sudo/wheel privileges can access this dashboard." })),
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

// ── Token Refresh ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub token: String,
}

/// POST /api/auth/refresh — verifikasi token lama yang masih valid, return JWT baru dengan expiry diperpanjang
pub async fn refresh_token_handler(
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "changeme-jwt-secret".to_string());

    // Verifikasi token lama — hanya refresh jika token masih valid
    let claims = verify_token(&payload.token, &secret).map_err(|_| (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "Invalid or expired token, please login again" })),
    ))?;

    // Buat token baru dengan username + password yang sama, expiry baru 24 jam
    match create_token(&claims.sub, &claims.pwd, &secret) {
        Ok(token) => Ok(Json(LoginResponse { token, username: claims.sub })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Token refresh failed: {}", e) })),
        )),
    }
}


