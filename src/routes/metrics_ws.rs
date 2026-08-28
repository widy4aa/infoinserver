use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use tokio::time::{interval, Duration};
use crate::AppState;
use crate::auth::jwt::verify_token;
use crate::services::proc_reader::get_system_metrics;
use crate::services::process_info::get_top_processes;

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

pub async fn metrics_ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "changeme-jwt-secret".to_string());
    
    // Verifikasi Token langsung di handler (karena tidak lewat JWT middleware reguler)
    if verify_token(&query.token, &secret).is_err() {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }

    ws.on_upgrade(move |socket| handle_metrics_socket(socket, state))
}

async fn handle_metrics_socket(mut socket: WebSocket, state: AppState) {
    let mut ticker = interval(Duration::from_secs(3));

    loop {
        ticker.tick().await;

        let sys_metrics = get_system_metrics();
        
        let processes = {
            let mut sys_lock = state.sys.lock().unwrap();
            get_top_processes(&mut sys_lock)
        };

        let payload = serde_json::json!({
            "type": "metrics_update",
            "system": sys_metrics,
            "processes": processes,
        });

        if let Ok(json_str) = serde_json::to_string(&payload) {
            if socket.send(Message::Text(json_str.into())).await.is_err() {
                // Client disconnected
                break;
            }
        } else {
            break;
        }
    }
}
