// src/auth/jwt_middleware.rs
// Axum middleware — intercept semua request /api/* kecuali /api/auth/login
// Inject Claims ke request extensions agar handler bisa ambil username + password untuk sudo

use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use crate::auth::jwt::{verify_token, Claims};

/// Extension type — diinject ke request oleh middleware
#[derive(Clone, Debug)]
pub struct AuthUser(pub Claims);

/// Middleware JWT — wajib dipasang di semua route /api/* kecuali /api/auth/*
pub async fn jwt_auth_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let path = request.uri().path().to_string();

    // Whitelist: endpoint auth & websocket tidak perlu token langsung dari header
    // (WebSocket endpoints menggunakan ?token= query param, divalidasi di bawah)
    if path.starts_with("/api/auth/") 
        || path.starts_with("/api/metrics/ws")
        || path.starts_with("/api/cloudflare/logs/ws") {
        return Ok(next.run(request).await);
    }

    // Ambil JWT dari header Authorization: Bearer <token>
    // ATAU dari query parameter ?token=<token> (untuk WebSocket)
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|v| v.to_string());

    let token = match token {
        Some(t) => t,
        None => {
            // Coba ambil dari query param
            let query = request.uri().query().unwrap_or("");
            let q_token = query.split('&').find(|p| p.starts_with("token=")).map(|p| p.replace("token=", ""));
            
            match q_token {
                Some(t) if !t.is_empty() => t,
                _ => {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        axum::Json(serde_json::json!({
                            "error": "Missing Authorization header or token query param"
                        })),
                    ));
                }
            }
        }
    };

    // Ambil JWT_SECRET dari AppState atau env
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "changeme-jwt-secret".to_string());

    // Verify token
    match verify_token(&token, &secret) {
        Ok(claims) => {
            // Inject AuthUser ke extensions agar handler bisa akses
            request.extensions_mut().insert(AuthUser(claims));
            Ok(next.run(request).await)
        }
        Err(_) => Err((
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "Invalid or expired token"
            })),
        )),
    }
}
