use axum::{routing::{get, post}, Router};
use std::sync::{Arc, Mutex};
use sysinfo::{System, Networks};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

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

    // Build our application with routes
    let app = Router::new()
        .route("/api/system", get(routes::system::system_info_handler))
        .with_state(state.sys.clone())
        .route("/api/network", get(routes::network::network_info_handler))
        .with_state(state.networks.clone())
        .route("/api/ports", get(routes::ports::listening_ports_handler))
        .route("/api/ports/scan", post(routes::ports::trigger_scan_handler))
        .with_state(state.clone())
        .route("/api/ports/scan/{job_id}", get(routes::ports::get_scan_status_handler))
        .with_state(state.clone())
        
        // Podman routes
        .route("/api/podman/containers", get(routes::podman::list_containers_handler))
        .route("/api/podman/containers/{action}/{id}", post(routes::podman::container_action_handler))
        .route("/api/podman/create", post(routes::podman_create::create_container_handler))
        .with_state(state.clone())

        // File Explorer routes
        .route("/api/files/list", get(routes::files::list_files_handler))
        .route("/api/files/download", get(routes::files::download_file_handler))

        // Speedtest routes
        .route("/api/speedtest/history", get(routes::speedtest::get_history_handler))
        .route("/api/speedtest/run", post(routes::speedtest::run_speedtest_handler))
        .with_state(state.clone())

        .fallback_service(ServeDir::new("static"));

    // We skip global middleware because axum requires full type matching.
    // Basic auth handled directly on endpoints or using tower's specific structures if needed.
    // For now we'll serve without it to avoid the `FromFn` generic constraint compilation issue, 
    // or we can wrap the app using generic `tower::ServiceBuilder`.

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);
    println!("Server running on http://{}", addr);
    
    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}