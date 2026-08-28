use axum::{extract::{Extension, Query}, Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;

#[derive(Deserialize)]
pub struct LogsQuery {
    pub filter: String, // all, auth, kernel
}

#[derive(Serialize)]
pub struct SyslogResponse {
    pub logs: String,
}

pub async fn get_syslogs_handler(
    Query(query): Query<LogsQuery>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<SyslogResponse>, (StatusCode, Json<serde_json::Value>)> {
    
    let mut args = vec!["journalctl", "-n", "300", "--no-pager", "-o", "short-iso", "--reverse"];
    
    let filter_str = query.filter.as_str();
    match filter_str {
        "auth" => {
            args.push("-u");
            args.push("sshd"); // or auth.log fallback
        },
        "kernel" => {
            args.push("-k");
        },
        "all" | _ => {
            // standard all logs
        }
    }

    let p = auth.0.pwd;

    let out = tokio::task::spawn_blocking(move || {
        sudo_exec(&p, &args)
    }).await.unwrap()
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    if out.status.success() {
        let logs = String::from_utf8_lossy(&out.stdout).to_string();
        Ok(Json(SyslogResponse { logs }))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("journalctl failed: {}", err)}))))
    }
}
