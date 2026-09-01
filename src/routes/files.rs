use axum::{
    extract::{Query, Multipart, Extension},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::process::Command;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use crate::auth::jwt_middleware::AuthUser;
use crate::services::file_manager::{
    list_directory, resolve_and_validate_path, resolve_path_safe,
    check_write_permission, get_removable_mounts, FileInfo
};

#[derive(Deserialize)]
pub struct FileQuery {
    pub path: Option<String>,
}

#[derive(Deserialize)]
pub struct FetchUrlRequest {
    pub url: String,
    pub path: String,
}

/// Helper: dapatkan home_root berdasarkan username dari session JWT.
/// Jika FILE_ROOT=$HOME, expand ke /home/<username> (bukan env $HOME backend).
/// Ini memungkinkan user switch menggunakan file explorer di home directory mereka masing-masing.
fn get_home_root(username: &str) -> String {
    let val = env::var("FILE_ROOT").unwrap_or_else(|_| "$HOME".to_string());
    if val == "$HOME" || val.starts_with("$HOME/") {
        // Expand $HOME ke home directory user yang sedang login di session
        // bukan $HOME dari environment variable backend OS
        let user_home = if username.is_empty() {
            // Fallback ke env $HOME jika username tidak tersedia
            env::var("HOME").unwrap_or_else(|_| "/root".to_string())
        } else {
            format!("/home/{}", username)
        };
        val.replace("$HOME", &user_home)
    } else {
        val
    }
}

/// Endpoint untuk membaca konfigurasi file manager (dibutuhkan frontend untuk tentukan read-only)
pub async fn get_files_config_handler(
    Extension(auth): Extension<AuthUser>,
) -> Json<serde_json::Value> {
    let username = auth.0.sub.clone();
    let home_root = get_home_root(&username);
    Json(serde_json::json!({
        "home_root": home_root,
        "system_root": "/"
    }))
}

pub async fn list_files_handler(
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<FileQuery>,
) -> Result<Json<Vec<FileInfo>>, (StatusCode, String)> {
    let home_root = get_home_root(&auth.0.sub);
    let req_path = query.path.unwrap_or_else(|| "/".to_string());

    // Izinkan akses ke seluruh filesystem (Opsi A)
    match resolve_path_safe(&req_path) {
        Ok(valid_path) => {
            match list_directory(&valid_path, &home_root) {
                Ok(files) => Ok(Json(files)),
                Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
            }
        },
        Err(e) => Err((StatusCode::FORBIDDEN, e)),
    }
}

pub async fn download_file_handler(Query(query): Query<FileQuery>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let req_path = query.path.unwrap_or_default();

    if req_path.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Path parameter is required".to_string()));
    }

    // Download adalah read-only, boleh dari mana saja
    let valid_path = resolve_path_safe(&req_path)
        .map_err(|e| (StatusCode::FORBIDDEN, e))?;

    if valid_path.is_dir() {
        return Err((StatusCode::BAD_REQUEST, "Cannot download a directory directly".to_string()));
    }

    let file = match File::open(&valid_path).await {
        Ok(file) => file,
        Err(_) => return Err((StatusCode::NOT_FOUND, "File not found".to_string())),
    };

    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    let filename = valid_path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let content_disposition = format!("attachment; filename=\"{}\"", filename);

    let headers = [
        (header::CONTENT_TYPE, "application/octet-stream".to_string()),
        (header::CONTENT_DISPOSITION, content_disposition),
    ];

    Ok((headers, body))
}

pub async fn upload_file_handler(
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<FileQuery>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let home_root = get_home_root(&auth.0.sub);
    let req_path = query.path.unwrap_or_else(|| "/".to_string());
    
    // Upload wajib cek write permission
    let valid_dir = resolve_path_safe(&req_path)
        .map_err(|e| (StatusCode::FORBIDDEN, e))?;

    let removable_mounts = get_removable_mounts();
    if !check_write_permission(&valid_dir, &home_root, &removable_mounts) {
        return Err((StatusCode::FORBIDDEN, "Path is read-only (outside home and removable drives)".to_string()));
    }

    if !valid_dir.is_dir() {
        return Err((StatusCode::BAD_REQUEST, "Upload target path must be a directory".to_string()));
    }

    let mut uploaded_count = 0;

    // Baca setiap file yang diupload (dukung multi upload)
    while let Some(field) = multipart.next_field().await.map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {}", e)))? {
        // Ambil nama asli file
        let file_name = if let Some(file_name) = field.file_name() {
            file_name.to_string()
        } else {
            continue; // Skip jika tidak ada nama file
        };

        // Cegah path traversal dari nama file itu sendiri
        if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
            return Err((StatusCode::BAD_REQUEST, "Invalid file name".to_string()));
        }

        let file_path = valid_dir.join(&file_name);
        
        let data = field.bytes().await.map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read multipart data: {}", e)))?;

        // Tulis file
        let mut file = File::create(&file_path)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create file: {}", e)))?;
            
        file.write_all(&data)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write file: {}", e)))?;

        uploaded_count += 1;
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": format!("{} file(s) uploaded successfully", uploaded_count)
    })))
}

pub async fn fetch_url_handler(
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<FetchUrlRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let home_root = get_home_root(&auth.0.sub);
    
    let valid_dir = resolve_path_safe(&payload.path)
        .map_err(|e| (StatusCode::FORBIDDEN, e))?;

    let removable_mounts = get_removable_mounts();
    if !check_write_permission(&valid_dir, &home_root, &removable_mounts) {
        return Err((StatusCode::FORBIDDEN, "Path is read-only".to_string()));
    }

    if !valid_dir.is_dir() {
        return Err((StatusCode::BAD_REQUEST, "Target path must be a directory".to_string()));
    }

    // Eksekusi wget
    let output = Command::new("wget")
        .current_dir(&valid_dir)
        .args(["-q", "-nc", &payload.url]) // -q quiet, -nc no-clobber (jangan timpa file jika sudah ada)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to spawn wget: {}", e)))?;

    if output.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "File fetched from URL successfully"
        })))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, "Wget failed to download the file. Ensure the URL is valid and wget is installed.".to_string()))
    }
}
// struct tambahan untuk File Actions
#[derive(Deserialize)]
pub struct FileActionRequest {
    pub action: String, // "rename", "move", "copy", "delete", "compress", "extract", "chmod"
    pub target: String, // File atau folder utama
    pub destination: Option<String>, // Nama baru (rename), atau path tujuan (move/copy), atau nama file zip (compress)
    pub password: Option<String>, // Untuk extract zip
}

#[derive(Deserialize)]
pub struct FileTextRequest {
    pub path: String,
    pub content: Option<String>, // Jika ada isinya, berarti WRITE. Jika kosong, berarti READ.
}
pub async fn file_action_handler(
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<FileActionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let home_root = get_home_root(&auth.0.sub);
    let removable_mounts = get_removable_mounts();
    
    // Resolve target path
    let valid_target = resolve_path_safe(&payload.target)
        .map_err(|e| (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": e}))))?;

    // READ actions (delete tidak butuh check di target karena delete = modifikasi)
    // Untuk aksi modifikasi: wajib check write permission pada TARGET
    let write_actions = ["rename", "move", "copy", "delete", "compress", "extract", "chmod"];
    if write_actions.contains(&payload.action.as_str()) {
        // Untuk copy: source boleh read-only, tapi destination wajib writable
        if payload.action == "copy" {
            // Source tidak wajib writable, destination wajib writable
        } else {
            if !check_write_permission(&valid_target, &home_root, &removable_mounts) {
                return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Path is read-only (outside home and removable drives)"}))));
            }
        }
    }

    // Beberapa action butuh destination
    let valid_dest = if let Some(dest) = &payload.destination {
        if payload.action == "rename" {
            let parent = valid_target.parent().unwrap();
            let new_path = parent.join(dest);
            if dest.contains('/') || dest.contains('\\') || dest.contains("..") {
                return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid rename destination"}))));
            }
            Some(new_path)
        } else if payload.action == "chmod" {
            None
        } else {
            // Move, Copy, Compress, Extract — pakai absolute path
            let res = resolve_path_safe(dest)
                .map_err(|e| (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": e}))))?;
            // Untuk copy/move: destination wajib writable
            if payload.action == "copy" || payload.action == "move" {
                if !check_write_permission(&res, &home_root, &removable_mounts) {
                    return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Destination is read-only"}))));
                }
            }
            Some(res)
        }
    } else {
        None
    };

    let action_str = payload.action.as_str();

    let output = match action_str {
        "rename" | "move" => {
            let dest = valid_dest.ok_or((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Destination required"}))))?;
            Command::new("mv").arg(&valid_target).arg(&dest).output()
        },
        "copy" => {
            let dest = valid_dest.ok_or((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Destination required"}))))?;
            Command::new("cp").arg("-r").arg(&valid_target).arg(&dest).output()
        },
        "delete" => {
            // rm -rf sangat berbahaya, pastikan target aman
            Command::new("rm").arg("-rf").arg(&valid_target).output()
        },
        "chmod" => {
            let perms = payload.destination.as_deref().unwrap_or("");
            if !perms.chars().all(|c| c.is_ascii_digit()) || perms.is_empty() {
                return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid permissions format (e.g. 755)"}))));
            }
            Command::new("chmod").arg(perms).arg(&valid_target).output()
        },
        "compress" => {
            let dest = valid_dest.ok_or((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Destination zip file required"}))))?;
            let parent_dir = valid_target.parent().unwrap();
            let file_name = valid_target.file_name().unwrap();
            
            // cd ke parent dir agar path di zip relatif
            Command::new("zip")
                .current_dir(parent_dir)
                .arg("-r")
                .arg(&dest)
                .arg(file_name)
                .output()
        },
        "extract" => {
            let dest = valid_dest.ok_or((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Destination folder required"}))))?;
            let mut cmd = Command::new("unzip");
            
            if let Some(pass) = &payload.password {
                if !pass.is_empty() {
                    cmd.arg("-P").arg(pass);
                }
            }
            
            cmd.arg(&valid_target).arg("-d").arg(&dest).output()
        },
        _ => return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid action"})))),
    };

    let result = output.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to spawn command: {}", e)}))))?;

    if result.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Action '{}' completed successfully", action_str)
        })))
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Command failed: {}", stderr)}))))
    }
}

pub async fn text_file_handler(
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<FileTextRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let home_root = get_home_root(&auth.0.sub);
    
    let valid_path = resolve_path_safe(&payload.path)
        .map_err(|e| (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": e}))))?;

    if valid_path.is_dir() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Cannot open a directory as text"}))));
    }

    if let Some(content) = payload.content {
        // WRITE Mode — cek permission dulu
        let removable_mounts = get_removable_mounts();
        if !check_write_permission(&valid_path, &home_root, &removable_mounts) {
            return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "File is read-only (outside home and removable drives)"}))));
        }
        tokio::fs::write(&valid_path, content).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to write file: {}", e)}))))?;
            
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "File saved successfully"
        })))
    } else {
        // READ Mode — boleh dari mana saja
        let text = tokio::fs::read_to_string(&valid_path).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to read file (might be binary): {}", e)}))))?;
            
        Ok(Json(serde_json::json!({
            "status": "success",
            "content": text
        })))
    }
}

#[derive(Serialize)]
pub struct DetailedFileInfo {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub permissions_octal: String, // e.g. "0755"
    pub permissions_symbolic: String, // e.g. "rwxr-xr-x"
    pub modified_at: u64, // Unix timestamp
    pub owner: String,   // "user:group"
}

pub async fn file_info_handler(Query(query): Query<FileQuery>) -> Result<Json<DetailedFileInfo>, (StatusCode, Json<serde_json::Value>)> {
    let req_path = query.path.unwrap_or_default();

    if req_path.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Path parameter is required"}))));
    }

    // file_info adalah read-only, boleh dari mana saja
    let valid_path = resolve_path_safe(&req_path)
        .map_err(|e| (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": e}))))?;

    let metadata = tokio::fs::metadata(&valid_path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Cannot read metadata: {}", e)}))))?;

    use std::os::unix::fs::PermissionsExt;
    let mode = metadata.permissions().mode();
    let permissions_octal = format!("{:04o}", mode & 0o7777);

    // Build symbolic permissions string (e.g. "rwxr-xr-x")
    let symbolic = mode_to_symbolic(mode);

    // Modified time
    let modified_at = metadata.modified()
        .unwrap_or(std::time::UNIX_EPOCH)
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Owner via stat command
    let owner = Command::new("stat")
        .args(["--printf=%U:%G", valid_path.to_str().unwrap_or("")])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|| "unknown:unknown".to_string());

    Ok(Json(DetailedFileInfo {
        path: req_path,
        name: valid_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        is_dir: metadata.is_dir(),
        size_bytes: metadata.len(),
        permissions_octal,
        permissions_symbolic: symbolic,
        modified_at,
        owner,
    }))
}

fn mode_to_symbolic(mode: u32) -> String {
    let types = [
        (0o400, 'r'), (0o200, 'w'), (0o100, 'x'),  // owner
        (0o040, 'r'), (0o020, 'w'), (0o010, 'x'),  // group
        (0o004, 'r'), (0o002, 'w'), (0o001, 'x'),  // others
    ];
    let mut s = String::new();
    for (bit, ch) in &types {
        if mode & bit != 0 { s.push(*ch); } else { s.push('-'); }
    }
    s
}

// ── SEARCH ──

#[derive(Deserialize)]
pub struct SearchQuery {
    pub path: Option<String>,
    pub query: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub name: String,
    pub full_path: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub size: u64,
    pub writable: bool,
}

pub async fn search_files_handler(
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, (StatusCode, String)> {
    let search_term = query.query.trim().to_string();
    if search_term.is_empty() || search_term.len() < 2 {
        return Err((StatusCode::BAD_REQUEST, "Search query must be at least 2 characters".to_string()));
    }

    let base_path = query.path.unwrap_or_else(|| "/".to_string());
    let valid_base = resolve_path_safe(&base_path)
        .map_err(|e| (StatusCode::FORBIDDEN, e))?;

    let home_root = get_home_root(&auth.0.sub);
    let removable_mounts = get_removable_mounts();

    // Gunakan 'find' command — exclude /proc dan /sys yang bisa hang
    let pattern = format!("*{}*", search_term);
    let out = Command::new("find")
        .args([
            valid_base.to_str().unwrap_or("/"),
            "-maxdepth", "8",
            "-iname", &pattern,
            "-not", "-path", "*/proc/*",
            "-not", "-path", "*/sys/*",
            "-not", "-path", "*/.git/*",
            "-not", "-path", "*/node_modules/*",
        ])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Search failed: {}", e)))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let base_prefix = format!("{}/", valid_base.display());

    let mut results = Vec::new();
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let full_path = line.trim().to_string();
        let path = std::path::Path::new(&full_path);

        if let Ok(meta) = std::fs::metadata(path) {
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let relative_path = full_path.strip_prefix(&base_prefix)
                .unwrap_or(&full_path)
                .to_string();
            let writable = check_write_permission(path, &home_root, &removable_mounts);

            results.push(SearchResult {
                name,
                full_path,
                relative_path,
                is_dir: meta.is_dir(),
                size: meta.len(),
                writable,
            });
        }

        // Batasi 500 hasil agar tidak overflow
        if results.len() >= 500 {
            break;
        }
    }

    // Sort: folder dulu, lalu file
    results.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));

    Ok(Json(results))
}
