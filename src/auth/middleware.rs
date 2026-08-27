use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
    response::IntoResponse,
};
use std::env;
use axum_auth::AuthBasic;

#[allow(dead_code)]
pub async fn require_auth(
    auth: Option<AuthBasic>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let expected_user = env::var("AUTH_USER").unwrap_or_else(|_| "admin".to_string());
    let expected_pass = env::var("AUTH_PASS").unwrap_or_else(|_| "admin123".to_string());

    let mut is_authorized = false;

    if let Some(AuthBasic((id, password))) = auth {
        if id == expected_user {
            if let Some(pass) = password {
                if pass == expected_pass {
                    is_authorized = true;
                }
            }
        }
    }

    if is_authorized {
        Ok(next.run(request).await.into_response())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}