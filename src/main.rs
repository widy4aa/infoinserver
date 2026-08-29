use axum::{routing::{get, post, delete, put}, Router, middleware};
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
    // Install ring sebagai rustls crypto provider (pure Rust, tidak butuh libclang/openssl)
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt::init();
    
    let _ = dotenvy::dotenv();

    let db_url = std::env::var("DB_PATH").unwrap_or_else(|_| "sqlite:./data.db".to_string());
    let db_pool = db::init_db(&db_url).await.expect("Failed to initialize database");

    let state = AppState {
        sys: Arc::new(Mutex::new(System::new_all())),
        networks: Arc::new(Mutex::new(Networks::new_with_refreshed_list())),
        db_pool: db_pool.clone(),
    };

    background::scheduler::start_background_tasks(db_pool).await;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // ── Public routes (tidak perlu auth) ─────────────────────
    let public_routes = Router::new()
        .route("/api/auth/login", post(routes::auth::login_handler));

    // ── Protected routes (semua wajib JWT) ───────────────────
    let protected_routes = Router::new()
        // Hapus route polling system
        // .route("/api/system", get(routes::system::system_info_handler))
        // .route("/api/process/list", get(routes::process_mgmt::list_processes_handler))
        .route("/api/metrics/ws", get(routes::metrics_ws::metrics_ws_handler))
        .with_state(state.clone())
        .route("/api/metrics/history", get(routes::metrics_history::get_metrics_history_handler))
        .with_state(state.clone())
        .route("/api/logs/activity", get(routes::logs::get_activity_logs_handler))
        .with_state(state.clone())
        .route("/api/system/update", post(routes::system_mgmt::update_dashboard_handler))
        .route("/api/system/reboot", post(routes::system_mgmt::reboot_server_handler))
        .route("/api/users", get(routes::users_mgmt::get_users_handler))
        .route("/api/users", post(routes::users_mgmt::create_user_handler))
        .route("/api/users/{username}/password", put(routes::users_mgmt::change_password_handler))
        .route("/api/users/{username}/groups", put(routes::users_mgmt::update_user_groups_handler))
        .route("/api/users/{username}", delete(routes::users_mgmt::delete_user_handler))
        .route("/api/groups", get(routes::users_mgmt::get_groups_handler))
        .route("/api/services", get(routes::services_mgmt::list_services_handler))
        .route("/api/services/action", post(routes::services_mgmt::service_action_handler))
        .route("/api/syslogs", get(routes::syslogs::get_syslogs_handler))
        .route("/api/cron", get(routes::cron_mgmt::get_cron_handler))
        .route("/api/cron", post(routes::cron_mgmt::update_cron_handler))
        .route("/api/network", get(routes::network::network_info_handler))
        .with_state(state.networks.clone())
        .route("/api/ports", get(routes::ports::listening_ports_handler))
        .route("/api/ports/scan", post(routes::ports::trigger_scan_handler))
        .with_state(state.clone())
        .route("/api/ports/scan/{job_id}", get(routes::ports::get_scan_status_handler))
        .with_state(state.clone())
        .route("/api/process/list", get(routes::process_mgmt::list_processes_handler))
        .with_state(state.clone())
        .route("/api/process/kill/{pid}", post(routes::process_mgmt::kill_process_handler))
        .route("/api/podman/containers", get(routes::podman::list_containers_handler))
        .route("/api/podman/containers/{action}/{id}", post(routes::podman::container_action_handler))
        .route("/api/podman/create", post(routes::podman_create::create_container_handler))
        .route("/api/podman/inspect/{id}", get(routes::podman_details::get_container_details_handler))
        .route("/api/podman/logs/{id}", get(routes::podman_logs::get_container_logs_handler))
        .with_state(state.clone())
        .route("/api/files/list", get(routes::files::list_files_handler))
        .route("/api/files/download", get(routes::files::download_file_handler))
        .route("/api/files/upload", post(routes::files::upload_file_handler))
        .route("/api/files/fetch", post(routes::files::fetch_url_handler))
        .route("/api/files/action", post(routes::files::file_action_handler))
        .route("/api/files/text", post(routes::files::text_file_handler))
        .route("/api/files/info", get(routes::files::file_info_handler))
        .route("/api/speedtest/history", get(routes::speedtest::get_history_handler))
        .route("/api/speedtest/run", post(routes::speedtest::run_speedtest_handler))
        .with_state(state.clone())
        .route("/api/firewall/status", get(routes::firewall::get_ufw_status_handler))
        .route("/api/firewall/toggle", post(routes::firewall::toggle_ufw_handler))
        .route("/api/firewall/rule", post(routes::firewall::manage_ufw_rule_handler))
        // ── Cloudflare Local Management ────────────────────────
        .route("/api/cloudflare/status", get(routes::cloudflare::get_cloudflare_status))
        .route("/api/cloudflare/install", post(routes::cloudflare::install_cloudflared))
        .route("/api/cloudflare/create", post(routes::cloudflare::create_tunnel))
        .route("/api/cloudflare/login", post(routes::cloudflare::start_tunnel_login))
        .route("/api/cloudflare/login/status", get(routes::cloudflare::check_login_status))
        .route("/api/cloudflare/stop", post(routes::cloudflare::stop_cloudflare_tunnel))
        .route("/api/cloudflare/restart", post(routes::cloudflare_api::restart_service))
        .route("/api/cloudflare/config", get(routes::cloudflare_api::get_local_config))
        .route("/api/cloudflare/routes", post(routes::cloudflare_api::add_local_route))
        .route("/api/cloudflare/routes", delete(routes::cloudflare_api::delete_local_route))
        .route("/api/terminal/start", post(routes::terminal::start_shellinabox_handler))
        .route("/api/terminal/ws", get(routes::terminal_ws::terminal_ws_handler))
        // Apply JWT middleware ke semua protected routes
        .layer(middleware::from_fn(auth::jwt_middleware::jwt_auth_middleware));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .fallback_service(ServeDir::new("static"))
        .layer(cors);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    println!("Server running on http://{}", addr);
    
    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
