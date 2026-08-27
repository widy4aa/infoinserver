use axum::{extract::State, Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use reqwest::Client;
use crate::AppState;

#[derive(Serialize, Deserialize, FromRow, Clone)]
pub struct CloudflareApiConfig {
    pub account_id: String,
    pub tunnel_id: String,
    pub api_token: String,
}

#[derive(Deserialize)]
pub struct RouteRequest {
    pub hostname: String,
    pub service: String,
}

pub async fn get_cf_config(State(state): State<AppState>) -> Result<Json<Option<CloudflareApiConfig>>, (StatusCode, String)> {
    let config = sqlx::query_as::<_, CloudflareApiConfig>("SELECT account_id, tunnel_id, api_token FROM cloudflare_config WHERE id = 1")
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;
        
    Ok(Json(config))
}

pub async fn save_cf_config(
    State(state): State<AppState>,
    Json(payload): Json<CloudflareApiConfig>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    
    let _ = sqlx::query(
        "INSERT INTO cloudflare_config (id, account_id, tunnel_id, api_token) VALUES (1, ?, ?, ?) 
         ON CONFLICT(id) DO UPDATE SET account_id=excluded.account_id, tunnel_id=excluded.tunnel_id, api_token=excluded.api_token"
    )
    .bind(payload.account_id)
    .bind(payload.tunnel_id)
    .bind(payload.api_token)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save config: {}", e)))?;

    Ok(Json(serde_json::json!({"status": "success", "message": "Cloudflare API Configuration saved."})))
}

// Menarik (GET) Tunnel Routes dari API Cloudflare Zero Trust
pub async fn get_tunnel_routes(State(state): State<AppState>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let config = get_config_from_db(&state.db_pool).await?;
    
    let client = Client::new();
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/configurations",
        config.account_id, config.tunnel_id
    );

    let res = client.get(&url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Failed to reach Cloudflare: {}", e)))?;

    if !res.status().is_success() {
        let err_txt = res.text().await.unwrap_or_default();
        return Err((StatusCode::BAD_REQUEST, format!("Cloudflare API Error: {}", err_txt)));
    }

    let data: serde_json::Value = res.json().await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse JSON".to_string()))?;
    Ok(Json(data))
}

// Menambahkan atau menimpa rute ingress di Cloudflare
pub async fn add_tunnel_route(
    State(state): State<AppState>,
    Json(payload): Json<RouteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let config = get_config_from_db(&state.db_pool).await?;
    let client = Client::new();

    // 1. Ambil config Ingress yang sekarang ada dulu
    let get_url = format!("https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/configurations", config.account_id, config.tunnel_id);
    let current_config: serde_json::Value = client.get(&get_url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?
        .json()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Parse error".to_string()))?;

    // 2. Modifikasi JSON Ingress-nya
    let mut ingress = current_config["result"]["config"]["ingress"].as_array().cloned().unwrap_or_else(|| vec![]);
    
    // Cloudflare ingress array harus diakhiri dengan catch-all { "service": "http_status:404" }
    // Cari dan keluarkan catch-all
    ingress.retain(|r| r["service"] != "http_status:404");

    // Tambahkan rule baru dari user
    ingress.push(serde_json::json!({
        "hostname": payload.hostname,
        "service": payload.service
    }));

    // Masukkan lagi catch-all di paling bawah
    ingress.push(serde_json::json!({ "service": "http_status:404" }));

    // 3. Kirim kembali config (PUT)
    let put_payload = serde_json::json!({
        "config": {
            "ingress": ingress
        }
    });

    let res = client.put(&get_url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .json(&put_payload)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    if res.status().is_success() {
        Ok(Json(serde_json::json!({"status": "success", "message": format!("Route for {} created!", payload.hostname)})))
    } else {
        Err((StatusCode::BAD_REQUEST, format!("API Error: {}", res.text().await.unwrap_or_default())))
    }
}

// Menghapus route
pub async fn delete_tunnel_route(
    State(state): State<AppState>,
    Json(payload): Json<RouteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let config = get_config_from_db(&state.db_pool).await?;
    let client = Client::new();

    let get_url = format!("https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/configurations", config.account_id, config.tunnel_id);
    let current_config: serde_json::Value = client.get(&get_url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send().await.unwrap().json().await.unwrap();

    let mut ingress = current_config["result"]["config"]["ingress"].as_array().cloned().unwrap_or_default();
    
    // Buang rule yang hostname-nya sama
    ingress.retain(|r| r["hostname"] != payload.hostname);

    let put_payload = serde_json::json!({ "config": { "ingress": ingress } });

    let res = client.put(&get_url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .json(&put_payload)
        .send().await.unwrap();

    if res.status().is_success() {
        Ok(Json(serde_json::json!({"status": "success", "message": "Route deleted"})))
    } else {
        Err((StatusCode::BAD_REQUEST, "Failed to update Cloudflare".to_string()))
    }
}

// Helper fetch DB
async fn get_config_from_db(pool: &sqlx::SqlitePool) -> Result<CloudflareApiConfig, (StatusCode, String)> {
    sqlx::query_as::<_, CloudflareApiConfig>("SELECT account_id, tunnel_id, api_token FROM cloudflare_config WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?
        .ok_or((StatusCode::BAD_REQUEST, "Cloudflare API Configuration not set in Dashboard Config".to_string()))
}