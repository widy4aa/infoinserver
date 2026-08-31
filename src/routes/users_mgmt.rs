use axum::{extract::{Extension, Path, Query}, Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::fs;
use std::collections::HashMap;
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;

// --- DTOs ---

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInfo {
    pub username: String,
    pub uid: u32,
    pub gid: u32,
    pub home: String,
    pub shell: String,
    pub groups: Vec<String>,
    pub is_system: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GroupInfo {
    pub name: String,
    pub gid: u32,
    pub members: Vec<String>,
}

#[derive(Deserialize)]
pub struct CreateUserReq {
    pub username: String,
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct ChangePasswordReq {
    pub password: String,
}

#[derive(Deserialize)]
pub struct UpdateGroupsReq {
    pub groups: Vec<String>, // list of group names
}

#[derive(Deserialize)]
pub struct DeleteUserQuery {
    pub remove_home: Option<String>, // "true" or "false"
}

// --- Handlers ---

/// Mengambil daftar semua user dari /etc/passwd dan /etc/group
pub async fn get_users_handler() -> Result<Json<Vec<UserInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let passwd = fs::read_to_string("/etc/passwd")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Failed to read passwd: {}", e) }))))?;
    
    let group = fs::read_to_string("/etc/group")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Failed to read group: {}", e) }))))?;

    // Parse /etc/group untuk mapping gid -> group name, dan user -> groups
    let mut gid_to_name: HashMap<u32, String> = HashMap::new();
    let mut user_to_groups: HashMap<String, Vec<String>> = HashMap::new();

    for line in group.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 4 {
            let group_name = parts[0].to_string();
            if let Ok(gid) = parts[2].parse::<u32>() {
                gid_to_name.insert(gid, group_name.clone());
            }
            
            let members: Vec<&str> = parts[3].split(',').filter(|s| !s.is_empty()).collect();
            for member in members {
                let member_str = member.to_string();
                user_to_groups
                    .entry(member_str)
                    .or_insert_with(Vec::new)
                    .push(group_name.clone());
            }
        }
    }

    // Parse /etc/passwd
    let mut users = Vec::new();
    for line in passwd.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 7 {
            let username = parts[0].to_string();
            let uid = parts[2].parse::<u32>().unwrap_or(0);
            let gid = parts[3].parse::<u32>().unwrap_or(0);
            let home = parts[5].to_string();
            let shell = parts[6].to_string();

            // is_system = uid < 1000 && username != root (uid 0)
            // root biasanya dipertimbangkan sebagai non-system user (admin utama) di UI kita, atau minimal uid < 1000 itu system
            let is_system = uid < 1000 && uid != 0;

            let mut user_groups = user_to_groups.get(&username).cloned().unwrap_or_default();
            // Primary group biasanya tidak ada di parts[3] list member, kita tambahkan manual
            if let Some(primary_group_name) = gid_to_name.get(&gid) {
                if !user_groups.contains(primary_group_name) {
                    user_groups.push(primary_group_name.clone());
                }
            }
            user_groups.sort();
            user_groups.dedup();

            users.push(UserInfo {
                username,
                uid,
                gid,
                home,
                shell,
                groups: user_groups,
                is_system,
            });
        }
    }

    // Sort by UID
    users.sort_by(|a, b| a.uid.cmp(&b.uid));

    Ok(Json(users))
}

/// Mengambil daftar semua grup
pub async fn get_groups_handler() -> Result<Json<Vec<GroupInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let group = fs::read_to_string("/etc/group")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Failed to read group: {}", e) }))))?;

    let mut groups = Vec::new();
    for line in group.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 4 {
            let name = parts[0].to_string();
            let gid = parts[2].parse::<u32>().unwrap_or(0);
            let members = parts[3].split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();
            
            groups.push(GroupInfo { name, gid, members });
        }
    }

    groups.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(groups))
}

/// POST /api/users
pub async fn create_user_handler(
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<CreateUserReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !payload.username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Invalid username format" }))));
    }

    let pwd = auth.0.pwd;

    // 1. sudo useradd -m -s /bin/bash <username>
    let out_add = tokio::task::spawn_blocking({
        let p = pwd.clone();
        let u = payload.username.clone();
        move || sudo_exec(&p, &["useradd", "-m", "-s", "/bin/bash", &u])
    }).await.unwrap()
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?;

    if !out_add.status.success() {
        let err = String::from_utf8_lossy(&out_add.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("useradd failed: {}", err) }))));
    }

    // 2. Tambahkan pengguna baru ini ke dalam grup 'sudo' (atau 'wheel') secara default.
    // Jika tidak ada 'sudo', coba tambahkan ke 'wheel'.
    let _ = tokio::task::spawn_blocking({
        let p = pwd.clone();
        let u = payload.username.clone();
        move || {
            let res = sudo_exec(&p, &["usermod", "-aG", "sudo", &u]);
            // Fallback for systems using 'wheel' instead of 'sudo'
            if let Ok(out) = res {
                if !out.status.success() {
                    let _ = sudo_exec(&p, &["usermod", "-aG", "wheel", &u]);
                }
            }
        }
    }).await.unwrap();

    // 3. Jika ada password, set password: echo "username:password" | chpasswd
    if let Some(user_pass) = payload.password {
        if !user_pass.is_empty() {
            let creds = format!("{}:{}", payload.username, user_pass);
            let out_pass = tokio::task::spawn_blocking(move || {
                use std::process::{Command, Stdio};
                use std::io::Write;
                
                let mut child = Command::new("sudo")
                    .arg("-S")
                    .arg("-p")
                    .arg("")
                    .arg("chpasswd")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;
            
                if let Some(stdin) = child.stdin.as_mut() {
                    let _ = stdin.write_all(format!("{}\n", pwd).as_bytes());
                    let _ = stdin.write_all(creds.as_bytes());
                }
            
                child.wait_with_output()
            }).await.unwrap()
              .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?;

            if !out_pass.status.success() {
                let err = String::from_utf8_lossy(&out_pass.stderr);
                return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("chpasswd failed: {}", err) }))));
            }
        }
    }

    Ok(Json(serde_json::json!({ "status": "success", "message": format!("User {} created & added to sudo/wheel", payload.username) })))
}

/// PUT /api/users/:username/password
pub async fn change_password_handler(
    Path(username): Path<String>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<ChangePasswordReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Invalid username" }))));
    }

    let pwd = auth.0.pwd;
    let creds = format!("{}:{}", username, payload.password);

    let out_pass = tokio::task::spawn_blocking(move || {
        use std::process::{Command, Stdio};
        use std::io::Write;
        let mut child = Command::new("sudo")
            .arg("-S")
            .arg("-p")
            .arg("")
            .arg("chpasswd")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(format!("{}\n", pwd).as_bytes());
            let _ = stdin.write_all(creds.as_bytes());
        }
        child.wait_with_output()
    }).await.unwrap()
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?;

    if out_pass.status.success() {
        Ok(Json(serde_json::json!({ "status": "success", "message": format!("Password for {} updated", username) })))
    } else {
        let err = String::from_utf8_lossy(&out_pass.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("chpasswd failed: {}", err) }))))
    }
}

/// PUT /api/users/:username/groups
pub async fn update_user_groups_handler(
    Path(username): Path<String>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<UpdateGroupsReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Invalid username" }))));
    }

    // Filter invalid group names
    let groups: Vec<String> = payload.groups.into_iter()
        .filter(|g| g.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .collect();

    let groups_str = groups.join(",");
    
    // usermod -G replaces all secondary groups with the new list
    let out = tokio::task::spawn_blocking({
        let p = auth.0.pwd;
        move || sudo_exec(&p, &["usermod", "-G", &groups_str, &username])
    }).await.unwrap()
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?;

    if out.status.success() {
        Ok(Json(serde_json::json!({ "status": "success", "message": "Groups updated" })))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("usermod failed: {}", err) }))))
    }
}

/// DELETE /api/users/:username
pub async fn delete_user_handler(
    Path(username): Path<String>,
    Query(query): Query<DeleteUserQuery>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Invalid username" }))));
    }

    let remove_home = query.remove_home.as_deref() == Some("true");
    
    let out = tokio::task::spawn_blocking({
        let p = auth.0.pwd;
        let u = username.clone();
        move || {
            if remove_home {
                sudo_exec(&p, &["userdel", "-r", &u])
            } else {
                sudo_exec(&p, &["userdel", &u])
            }
        }
    }).await.unwrap()
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?;

    if out.status.success() {
        Ok(Json(serde_json::json!({ "status": "success", "message": format!("User {} deleted", username) })))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("userdel failed: {}", err) }))))
    }
}

// ── SSH KEY MANAGER ──

#[derive(Deserialize)]
pub struct AddSshKeyRequest {
    pub key: String,
}

#[derive(Deserialize)]
pub struct DeleteSshKeyRequest {
    pub key: String,
}

fn get_authorized_keys_path(username: &str) -> String {
    if username == "root" {
        "/root/.ssh/authorized_keys".to_string()
    } else {
        format!("/home/{}/.ssh/authorized_keys", username)
    }
}

pub async fn get_ssh_keys_handler(
    Path(username): Path<String>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    let path = get_authorized_keys_path(&username);

    let out = tokio::task::spawn_blocking(move || {
        sudo_exec(&password, &["cat", &path])
    }).await.unwrap();

    // Jika file tidak ada, itu bukan error, berarti belum ada kunci
    match out {
        Ok(output) if output.status.success() => {
            let content = String::from_utf8_lossy(&output.stdout).to_string();
            let keys = content.lines().filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#')).map(|s| s.to_string()).collect();
            Ok(Json(keys))
        }
        _ => Ok(Json(vec![])),
    }
}

pub async fn add_ssh_key_handler(
    State(state): axum::extract::State<crate::AppState>,
    Path(username): Path<String>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<AddSshKeyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    let path = get_authorized_keys_path(&username);
    let key = payload.key.trim().to_string();

    if key.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Key cannot be empty".to_string()));
    }

    if !key.starts_with("ssh-rsa ") && !key.starts_with("ssh-ed25519 ") && !key.starts_with("ecdsa-sha2-nistp256 ") {
        return Err((StatusCode::BAD_REQUEST, "Invalid SSH key format. Must start with ssh-rsa, ssh-ed25519, etc.".to_string()));
    }

    // Pastikan folder .ssh ada
    let ssh_dir = if username == "root" { "/root/.ssh".to_string() } else { format!("/home/{}/.ssh", username) };
    
    let p = password.clone();
    let d = ssh_dir.clone();
    let _ = tokio::task::spawn_blocking(move || {
        let _ = sudo_exec(&p, &["mkdir", "-p", &d]);
        let _ = sudo_exec(&p, &["chmod", "700", &d]);
    }).await.unwrap();

    let p = password.clone();
    let out = tokio::task::spawn_blocking(move || {
        // Kita escape echo dengan aman
        let escaped = key.replace("'", "'\\''");
        let cmd = format!("echo '{}' >> {}", escaped, path);
        sudo_exec(&p, &["sh", "-c", &cmd])
    }).await.unwrap()
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if out.status.success() {
        // Pastikan hak akses file benar
        let p = password.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let _ = sudo_exec(&p, &["chmod", "600", &path]);
        }).await.unwrap();

        crate::routes::logs::log_activity(&state.db_pool, "WARNING", "SSH Key Added", &format!("Added SSH key to user {}", username)).await;
        Ok(Json(serde_json::json!({"status": "success", "message": "SSH key added successfully"})))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to add key: {}", err)))
    }
}

pub async fn delete_ssh_key_handler(
    State(state): axum::extract::State<crate::AppState>,
    Path(username): Path<String>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<DeleteSshKeyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();
    let path = get_authorized_keys_path(&username);
    let key_to_remove = payload.key.trim().to_string();

    let out = tokio::task::spawn_blocking(move || {
        sudo_exec(&password, &["cat", &path])
    }).await.unwrap();

    if let Ok(output) = out {
        if output.status.success() {
            let content = String::from_utf8_lossy(&output.stdout).to_string();
            // Filter keys
            let new_content: Vec<&str> = content.lines().filter(|l| l.trim() != key_to_remove).collect();
            let new_content_str = new_content.join("\n") + "\n";
            
            // Tulis kembali
            let p = auth.0.pwd.clone();
            let out2 = tokio::task::spawn_blocking(move || {
                let encoded = unsafe { String::from_utf8_unchecked(base64::encode(new_content_str).into_bytes()) };
                let cmd = format!("echo '{}' | base64 -d > {}", encoded, path);
                sudo_exec(&p, &["sh", "-c", &cmd])
            }).await.unwrap()
              .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            if out2.status.success() {
                crate::routes::logs::log_activity(&state.db_pool, "WARNING", "SSH Key Removed", &format!("Removed SSH key from user {}", username)).await;
                return Ok(Json(serde_json::json!({"status": "success", "message": "SSH key deleted successfully"})));
            } else {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to write updated authorized_keys".to_string()));
            }
        }
    }
    
    Err((StatusCode::NOT_FOUND, "authorized_keys not found or failed to read".to_string()))
}
