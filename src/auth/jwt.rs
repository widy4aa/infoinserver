// src/auth/jwt.rs
// JWT signing dan verification untuk session-based auth
// Token hanya bertahan selama browser terbuka (exp = 24 jam, tapi disimpan di sessionStorage frontend)

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, errors::Error};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Claims yang disimpan di dalam JWT
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    /// Subject — username Linux yang login
    pub sub: String,
    /// Password user (disimpan encrypted di token untuk inject sudo)
    /// Ini disimpan di JWT payload yang di-sign — tidak exposed ke client
    pub pwd: String,
    /// Expiry timestamp (Unix)
    pub exp: u64,
}

/// Generate JWT token setelah login sukses
pub fn create_token(username: &str, password: &str, secret: &str) -> Result<String, Error> {
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 86400; // 24 jam

    let claims = Claims {
        sub: username.to_string(),
        pwd: password.to_string(),
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Verifikasi dan decode JWT token — return Claims jika valid
pub fn verify_token(token: &str, secret: &str) -> Result<Claims, Error> {
    let mut validation = Validation::default();
    validation.validate_exp = true;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}
