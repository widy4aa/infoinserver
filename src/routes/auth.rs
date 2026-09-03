// src/routes/auth.rs
// POST /api/auth/login — verifikasi password Linux user via PAM, return JWT
// Hanya mengizinkan user non-root yang terdaftar di grup sudo/wheel
//
// GET /api/auth/github         — redirect ke GitHub OAuth consent screen
// GET /api/auth/github/callback — tukar code → GitHub token → profil → session token

use axum::{Json, http::StatusCode, response::Redirect, extract::Query};
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
    Query(params): Query<GithubCallbackQuery>,
) -> Result<Redirect, (StatusCode, Json<serde_json::Value>)> {
    // Jika user menolak di GitHub
    if let Some(err) = params.error {
        let frontend_url = format!("/login?error={}", urlencoding::encode(&err));
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
    let frontend_url = format!(
        "/auth/callback?token={}&user={}&name={}&avatar={}",
        urlencoding::encode(&session_token),
        urlencoding::encode(&github_user.login),
        urlencoding::encode(&display_name),
        urlencoding::encode(&github_user.avatar_url),
    );

    Ok(Redirect::temporary(&frontend_url))
}
