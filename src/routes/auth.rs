// src/routes/auth.rs
// POST /api/auth/login           — verifikasi password Linux user via PAM, return JWT
// GET  /api/auth/github          — redirect ke GitHub OAuth consent screen
// GET  /api/auth/github/callback — tukar code → GitHub token → profil → session token
// POST /api/auth/github/heartbeat — update last_seen user di DB
// GET  /api/auth/github/users    — return semua GitHub user + status online/offline

use axum::{Json, http::StatusCode, response::Redirect, extract::{Query, State}};
use serde::{Deserialize, Serialize};
use pam::Client;
use crate::auth::jwt::create_token;
use crate::AppState;
use std::time::{SystemTime, UNIX_EPOCH};

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

// ── GitHub OAuth ─────────────────────────────────────────────────────────────

/// GET /api/auth/github — redirect browser ke GitHub OAuth consent screen
pub async fn github_auth_handler() -> Redirect {
    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_default();
    let redirect_uri = std::env::var("GITHUB_REDIRECT_URI").unwrap_or_default();

    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read:user",
        client_id,
        urlencoding::encode(&redirect_uri)
    );

    Redirect::temporary(&url)
}

#[derive(Deserialize)]
pub struct GithubCallbackQuery {
    pub code: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct GithubTokenResponse {
    access_token: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct GithubUser {
    login: String,
    name: Option<String>,
    avatar_url: String,
}

/// GET /api/auth/github/callback — tukar code → access token → ambil profil → redirect frontend
pub async fn github_callback_handler(
    State(state): State<AppState>,
    Query(params): Query<GithubCallbackQuery>,
) -> Result<Redirect, (StatusCode, Json<serde_json::Value>)> {
    // Jika user menolak di GitHub
    if let Some(err) = params.error {
        let base = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let frontend_url = format!("{}/login?error={}", base, urlencoding::encode(&err));
        return Ok(Redirect::temporary(&frontend_url));
    }

    let code = params.code.ok_or_else(|| (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": "Missing code parameter" })),
    ))?;

    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = std::env::var("GITHUB_REDIRECT_URI").unwrap_or_default();

    // Tukar code → access token
    let client = reqwest::Client::new();
    let token_res = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "code": code,
            "redirect_uri": redirect_uri,
        }))
        .send()
        .await
        .map_err(|e| (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": format!("Failed to contact GitHub: {}", e) })),
        ))?;

    let token_data: GithubTokenResponse = token_res.json().await.map_err(|_| (
        StatusCode::BAD_GATEWAY,
        Json(serde_json::json!({ "error": "Invalid response from GitHub token endpoint" })),
    ))?;

    let access_token = token_data.access_token.ok_or_else(|| (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "GitHub did not return an access token" })),
    ))?;

    // Ambil profil GitHub user
    let user_res = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "infoinserver")
        .send()
        .await
        .map_err(|e| (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": format!("Failed to fetch GitHub user: {}", e) })),
        ))?;

    let github_user: GithubUser = user_res.json().await.map_err(|_| (
        StatusCode::BAD_GATEWAY,
        Json(serde_json::json!({ "error": "Invalid GitHub user response" })),
    ))?;

    // Buat session token sederhana — sign dengan GITHUB_SESSION_SECRET
    let session_secret = std::env::var("GITHUB_SESSION_SECRET")
        .unwrap_or_else(|_| "infoinserver-session-secret".to_string());

    let display_name = github_user.name.unwrap_or_else(|| github_user.login.clone());

    // Simpan/update user ke DB (upsert)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let _ = sqlx::query(
        "INSERT INTO github_users (username, name, avatar_url, last_seen)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(username) DO UPDATE SET name=excluded.name, avatar_url=excluded.avatar_url, last_seen=excluded.last_seen"
    )
    .bind(&github_user.login)
    .bind(&display_name)
    .bind(&github_user.avatar_url)
    .bind(now)
    .execute(&state.db_pool)
    .await;

    // Encode payload sebagai JSON lalu sign dengan HMAC-like approach menggunakan jsonwebtoken
    let session_claims = serde_json::json!({
        "sub": github_user.login,
        "name": display_name,
        "avatar": github_user.avatar_url,
        "exp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() + 86400 * 7, // 7 hari
    });

    let session_token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &session_claims,
        &jsonwebtoken::EncodingKey::from_secret(session_secret.as_bytes()),
    ).map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": format!("Session token error: {}", e) })),
    ))?;

    // Redirect ke frontend dengan data session di query params
    let base = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let frontend_url = format!(
        "{}/auth/callback?token={}&user={}&name={}&avatar={}",
        base,
        urlencoding::encode(&session_token),
        urlencoding::encode(&github_user.login),
        urlencoding::encode(&display_name),
        urlencoding::encode(&github_user.avatar_url),
    );

    Ok(Redirect::temporary(&frontend_url))
}

// ── GitHub Presence (Heartbeat + Users List) ──────────────────────────────────

#[derive(Deserialize)]
pub struct HeartbeatRequest {
    pub token: String,
}

/// POST /api/auth/github/heartbeat — update last_seen user dari session token
pub async fn github_heartbeat_handler(
    State(state): State<AppState>,
    Json(payload): Json<HeartbeatRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let session_secret = std::env::var("GITHUB_SESSION_SECRET")
        .unwrap_or_else(|_| "infoinserver-session-secret".to_string());

    // Verifikasi session token untuk extract username
    let mut validation = jsonwebtoken::Validation::default();
    validation.validate_exp = true;

    let token_data = jsonwebtoken::decode::<serde_json::Value>(
        &payload.token,
        &jsonwebtoken::DecodingKey::from_secret(session_secret.as_bytes()),
        &validation,
    ).map_err(|_| (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "Invalid session token" })),
    ))?;

    let username = token_data.claims
        .get("sub")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid token claims" })),
        ))?
        .to_string();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    sqlx::query("UPDATE github_users SET last_seen = ? WHERE username = ?")
        .bind(now)
        .bind(&username)
        .execute(&state.db_pool)
        .await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("DB error: {}", e) })),
        ))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Serialize)]
pub struct GithubUserPresence {
    pub username: String,
    pub name: String,
    pub avatar_url: String,
    pub online: bool,
    pub last_seen: i64,
}

/// GET /api/auth/github/users — return semua GitHub user + status online/offline
/// Online = last_seen dalam 60 detik terakhir
pub async fn github_users_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<GithubUserPresence>>, (StatusCode, Json<serde_json::Value>)> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let rows = sqlx::query_as::<_, (String, String, String, i64)>(
        "SELECT username, name, avatar_url, last_seen FROM github_users ORDER BY last_seen DESC"
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": format!("DB error: {}", e) })),
    ))?;

    let users = rows.into_iter().map(|(username, name, avatar_url, last_seen)| {
        let online = (now - last_seen) < 60;
        GithubUserPresence { username, name, avatar_url, online, last_seen }
    }).collect();

    Ok(Json(users))
}
