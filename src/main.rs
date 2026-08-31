use axum::{
    routing::{get, post, put, delete},
    Router, middleware,
};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

mod routes;
mod services;
mod background;
mod db;
mod auth;

use sysinfo::{System, Networks};

#[derive(Clone)]
pub struct AppState {
    sys: Arc<Mutex<System>>,
    networks: Arc<Mutex<Networks>>,
    db_pool: sqlx::SqlitePool,
}

#[derive(Clone)]
pub struct ContainerState {
    pub runtime: Arc<RwLock<Option<services::container_runtime::RuntimeInfo>>>,
}

#[tokio::main]
async fn main() {
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

    let runtime_info = match services::container_runtime::detect_runtime() {
        Ok(rt) => {
            println!("Container runtime: {} v{} (compose: {})", rt.binary, rt.version, rt.compose_binary);
            Some(rt)
        }
        Err(e) => {
            println!("Warning: No container runtime detected — {}", e);
            None
        }
    };

    let container_state = ContainerState {
        runtime: Arc::new(RwLock::new(runtime_info)),
    };

    background::scheduler::start_background_tasks(db_pool).await;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let public_routes = Router::new()
        .route("/api/ping", get(|| async { "pong" }))
        .route("/api/auth/login", post(routes::auth::login_handler));

    // ── Routes with ContainerState
    let container_routes = Router::new()
        .route("/api/container/runtime", get(routes::container::get_runtime_info_handler))
        .route("/api/container/runtime/refresh", post(routes::container::refresh_runtime_handler))
        .route("/api/container/list", get(routes::container::list_containers_handler))
        .route("/api/container/create", post(routes::container::create_container_handler))
        .route("/api/container/{action}/{id}", post(routes::container::container_action_handler))
        .route("/api/container/inspect/{id}", get(routes::container::inspect_handler))
        .route("/api/container/logs/{id}", get(routes::container::logs_handler))
        .route("/api/compose/projects", get(routes::compose::list_projects_handler))
        .route("/api/compose/deploy", post(routes::compose::deploy_project_handler))
        .route("/api/compose/{name}/stop", post(routes::compose::stop_project_handler))
        .route("/api/compose/{name}/restart", post(routes::compose::restart_project_handler))
        .route("/api/compose/{name}/rebuild", post(routes::compose::rebuild_project_handler))
        .route("/api/compose/{name}/ps", get(routes::compose::project_services_handler))
        .route("/api/compose/{name}/logs", get(routes::compose::project_logs_handler))
        .route("/api/compose/{name}/scale", post(routes::compose::scale_service_handler))
        .route("/api/compose/{name}/yaml", get(routes::compose::get_yaml_handler))
        .route("/api/compose/{name}/yaml", put(routes::compose::update_yaml_handler))
        .route("/api/compose/{name}", delete(routes::compose::delete_project_handler))
        .with_state(container_state);

    // ── Routes with Networks State
    let network_routes = Router::new()
        .route("/api/network", get(routes::network::network_info_handler))
        .with_state(state.networks.clone());

    // ── Routes with AppState
    let app_routes = Router::new()
        .route("/api/users", get(routes::users_mgmt::get_users_handler))
        .route("/api/users", post(routes::users_mgmt::create_user_handler))
        .route("/api/users/{username}/password", put(routes::users_mgmt::change_password_handler))
        .route("/api/users/{username}/groups", put(routes::users_mgmt::update_user_groups_handler))
        .route("/api/users/{username}", delete(routes::users_mgmt::delete_user_handler))
        .route("/api/users/{username}/ssh", get(routes::users_mgmt::get_ssh_keys_handler))
        .route("/api/groups", get(routes::users_mgmt::get_groups_handler))
        .route("/api/groups", post(routes::users_mgmt::create_group_handler))
        .route("/api/groups/{groupname}", delete(routes::users_mgmt::delete_group_handler))
        .route("/api/services", get(routes::services_mgmt::list_services_handler))
        .route("/api/services/action", post(routes::services_mgmt::service_action_handler))
        .route("/api/syslogs", get(routes::syslogs::get_syslogs_handler))
        .route("/api/cron", get(routes::cron_mgmt::get_cron_handler))
        .route("/api/cron", post(routes::cron_mgmt::update_cron_handler))
        .route("/api/ports", get(routes::ports::listening_ports_handler))
        .route("/api/files/list", get(routes::files::list_files_handler))
        .route("/api/files/download", get(routes::files::download_file_handler))
        .route("/api/files/upload", post(routes::files::upload_file_handler))
        .route("/api/files/fetch", post(routes::files::fetch_url_handler))
        .route("/api/files/action", post(routes::files::file_action_handler))
        .route("/api/files/text", post(routes::files::text_file_handler))
        .route("/api/files/info", get(routes::files::file_info_handler))
        .route("/api/speedtest/history", get(routes::speedtest::get_history_handler))
        .route("/api/cloudflare/status", get(routes::cloudflare::get_cloudflare_status))
        .route("/api/cloudflare/install", post(routes::cloudflare::install_cloudflared))
        .route("/api/cloudflare/login", post(routes::cloudflare::start_tunnel_login))
        .route("/api/cloudflare/login/status", get(routes::cloudflare::check_login_status))
        .route("/api/cloudflare/stop", post(routes::cloudflare::stop_cloudflare_tunnel))
        .route("/api/cloudflare/start", post(routes::cloudflare_api::start_service))
        .route("/api/cloudflare/restart", post(routes::cloudflare_api::restart_service))
        .route("/api/cloudflare/health", get(routes::cloudflare_api::check_health_status))
        .route("/api/cloudflare/logs", get(routes::cloudflare::get_cloudflare_logs))
        .route("/api/cloudflare/logs/ws", get(routes::cloudflare::cloudflare_logs_ws_handler))
        .route("/api/logs/bash_history", get(routes::logs::get_bash_history_handler))
        .route("/api/terminal/start", post(routes::terminal::start_shellinabox_handler))
        .route("/api/terminal/ws", get(routes::terminal_ws::terminal_ws_handler))
        // Stateful specific handler mappings
        .route("/api/users/{username}/ssh", post(routes::users_mgmt::add_ssh_key_handler))
        .route("/api/users/{username}/ssh", delete(routes::users_mgmt::delete_ssh_key_handler))
        .route("/api/metrics/ws", get(routes::metrics_ws::metrics_ws_handler))
        .route("/api/metrics/history", get(routes::metrics_history::get_metrics_history_handler))
        .route("/api/logs/activity", get(routes::logs::get_activity_logs_handler))
        .route("/api/system/update", post(routes::system_mgmt::update_dashboard_handler))
        .route("/api/system/os_updates", get(routes::system_updates::check_updates_handler))
        .route("/api/system/os_updates/ws", get(routes::system_updates::upgrade_ws_handler))
        .route("/api/system/reboot", post(routes::system_mgmt::reboot_server_handler))
        .route("/api/ports/scan", post(routes::ports::trigger_scan_handler))
        .route("/api/ports/scan/{job_id}", get(routes::ports::get_scan_status_handler))
        .route("/api/process/list", get(routes::process_mgmt::list_processes_handler))
        .route("/api/process/kill/{pid}", post(routes::process_mgmt::kill_process_handler))
        .route("/api/speedtest/run", post(routes::speedtest::run_speedtest_handler))
        .route("/api/firewall/status", get(routes::firewall::get_ufw_status_handler))
        .route("/api/firewall/toggle", post(routes::firewall::toggle_ufw_handler))
        .route("/api/firewall/rule", post(routes::firewall::manage_ufw_rule_handler))
        .route("/api/fail2ban/status", get(routes::fail2ban::get_status_handler))
        .route("/api/fail2ban/install", post(routes::fail2ban::install_handler))
        .route("/api/fail2ban/unban", post(routes::fail2ban::unban_ip_handler))
        .route("/api/fail2ban/ban", post(routes::fail2ban::ban_ip_handler))
        .route("/api/fail2ban/logs", get(routes::fail2ban::get_logs_handler))
        .route("/api/fail2ban/config", get(routes::fail2ban::get_jails_config_handler))
        .route("/api/fail2ban/config", post(routes::fail2ban::save_jail_config_handler))
        .route("/api/fail2ban/config/{name}", delete(routes::fail2ban::delete_jail_handler))
        .route("/api/fail2ban/filters", get(routes::fail2ban::get_filters_handler))
        .route("/api/cloudflare/create", post(routes::cloudflare::create_tunnel))
        .route("/api/cloudflare/tunnel", delete(routes::cloudflare::delete_tunnel))
        .route("/api/cloudflare/config", get(routes::cloudflare_api::get_local_config))
        .route("/api/cloudflare/routes", post(routes::cloudflare_api::add_local_route))
        .route("/api/cloudflare/routes", delete(routes::cloudflare_api::delete_local_route))
        .route("/api/cloudflare/routes/dns", post(routes::cloudflare_api::register_dns_cname))
        .with_state(state);

    let protected_routes = Router::new()
        .merge(container_routes)
        .merge(network_routes)
        .merge(app_routes)
        .route_layer(middleware::from_fn(auth::jwt_middleware::jwt_auth_middleware));

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