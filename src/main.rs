use axum::{routing::{get, post, delete}, Router};
use std::sync::{Arc, Mutex};
use sysinfo::{System, Networks};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tower_http::cors::{Any, CorsLayer};

mod routes;
mod services;
mod db;
mod background;
mod auth;

#[derive(Clone)]
pub struct AppState {
    sys: Arc<Mutex<System>>,
    networks: Arc<Mutex<Networks>>,
    db_pool: sqlx::SqlitePool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    // Load .env file explicitly
    let _ = dotenvy::dotenv();

    let db_url = std::env::var("DB_PATH").unwrap_or_else(|_| "sqlite:./data.db".to_string());
    let db_pool = db::init_db(&db_url).await.expect("Failed to initialize database");

    // Initialize state
    let state = AppState {
        sys: Arc::new(Mutex::new(System::new_all())),
        networks: Arc::new(Mutex::new(Networks::new_with_refreshed_list())),
        db_pool: db_pool.clone(),
    };

    // Start background tasks (e.g. speedtest scheduler)
    background::scheduler::start_background_tasks(db_pool).await;

    // Konfigurasi CORS agar frontend terpisah bisa memanggil backend ini
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build our application with routes
    let app = Router::new()
        .route("/api/system", get(routes::system::system_info_handler))
        .with_state(state.sys.clone())
        .route("/api/system/update", post(routes::system_mgmt::update_dashboard_handler))
        .route("/api/system/reboot", post(routes::system_mgmt::reboot_server_handler))
        .route("/api/network", get(routes::network::network_info_handler))
        .with_state(state.networks.clone())
        // Port & Process routes
        .route("/api/ports", get(routes::ports::listening_ports_handler))
        .route("/api/ports/scan", post(routes::ports::trigger_scan_handler))
        .with_state(state.clone())
        .route("/api/ports/scan/{job_id}", get(routes::ports::get_scan_status_handler))
        .with_state(state.clone())
        .route("/api/process/list", get(routes::process_mgmt::list_processes_handler))
        .with_state(state.clone())
        .route("/api/process/kill/{pid}", post(routes::process_mgmt::kill_process_handler))
        
        // Podman routes
        .route("/api/podman/containers", get(routes::podman::list_containers_handler))
        .route("/api/podman/containers/{action}/{id}", post(routes::podman::container_action_handler))
        .route("/api/podman/create", post(routes::podman_create::create_container_handler))
        .route("/api/podman/inspect/{id}", get(routes::podman_details::get_container_details_handler))
        .route("/api/podman/logs/{id}", get(routes::podman_logs::get_container_logs_handler))
        .with_state(state.clone())

        // File Explorer routes
        .route("/api/files/list", get(routes::files::list_files_handler))
        .route("/api/files/download", get(routes::files::download_file_handler))
        .route("/api/files/upload", post(routes::files::upload_file_handler))
        .route("/api/files/fetch", post(routes::files::fetch_url_handler))

        // Speedtest routes
        .route("/api/speedtest/history", get(routes::speedtest::get_history_handler))
        .route("/api/speedtest/run", post(routes::speedtest::run_speedtest_handler))
        .with_state(state.clone())

        // Firewall / UFW routes
        .route("/api/firewall/status", get(routes::firewall::get_ufw_status_handler))
        .route("/api/firewall/toggle", post(routes::firewall::toggle_ufw_handler))
        .route("/api/firewall/rule", post(routes::firewall::manage_ufw_rule_handler))

        // Cloudflare routes
        .route("/api/cloudflare/status", get(routes::cloudflare::get_cloudflare_status))
        .route("/api/cloudflare/install", post(routes::cloudflare::install_cloudflared))
        .route("/api/cloudflare/quick", post(routes::cloudflare::start_quick_tunnel))
        .route("/api/cloudflare/managed", post(routes::cloudflare::start_managed_tunnel))
        .route("/api/cloudflare/stop", post(routes::cloudflare::stop_cloudflare_tunnel))
        
        // Cloudflare Zero Trust API Routes
        .route("/api/cloudflare/api/config", get(routes::cloudflare_api::get_cf_config))
        .route("/api/cloudflare/api/config", post(routes::cloudflare_api::save_cf_config))
        .with_state(state.clone())
        .route("/api/cloudflare/api/routes", get(routes::cloudflare_api::get_tunnel_routes))
        .route("/api/cloudflare/api/routes", post(routes::cloudflare_api::add_tunnel_route))
        .route("/api/cloudflare/api/routes", delete(routes::cloudflare_api::delete_tunnel_route))
        .with_state(state.clone())

        // Terminal Route (Legacy Shellinabox)
        .route("/api/terminal/start", post(routes::terminal::start_shellinabox_handler))
        
        // Native WebSocket Terminal
        .route("/api/terminal/ws", get(routes::terminal_ws::terminal_ws_handler))

        .fallback_service(ServeDir::new("static"))
        .layer(cors);

    // We skip global middleware because axum requires full type matching.
    // Basic auth handled directly on endpoints or using tower's specific structures if needed.
    // For now we'll serve without it to avoid the `FromFn` generic constraint compilation issue, 
    // or we can wrap the app using generic `tower::ServiceBuilder`.

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port); // Ubah dari 127.0.0.1 ke 0.0.0.0 agar bisa diakses remote
    println!("Server running on http://{}", addr);
    
    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}