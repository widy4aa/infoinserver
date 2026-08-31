use axum::{
    extract::{Query, Multipart},
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

/// Helper: dapatkan home_root dari env
fn get_home_root() -> String {
    env::var("FILE_ROOT").unwrap_or_else(|_| "/".to_string())
}

/// Endpoint untuk membaca konfigurasi file manager (dibutuhkan frontend untuk tentukan read-only)
pub async fn get_files_config_handler() -> Json<serde_json::Value> {
    let home_root = get_home_root();
    Json(serde_json::json!({
        "home_root": home_root,
        "system_root": "/"
    }))
}

pub async fn list_files_handler(Query(query): Query<FileQuery>) -> Result<Json<Vec<FileInfo>>, (StatusCode, String)> {
    let home_root = get_home_root();
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
    Query(query): Query<FileQuery>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let home_root = get_home_root();
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

pub async fn fetch_url_handler(Json(payload): Json<FetchUrlRequest>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let home_root = get_home_root();
    
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
pub async fn file_action_handler(Json(payload): Json<FileActionRequest>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let home_root = get_home_root();
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

pub async fn text_file_handler(Json(payload): Json<FileTextRequest>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let home_root = get_home_root();
    
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
    // Ambil 3 digit terakhir (e.g. 755 dari 33261)
    let permissions_octal = format!("{:04o}", mode & 0o7777);

    Ok(Json(DetailedFileInfo {
        path: req_path,
        name: valid_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        is_dir: metadata.is_dir(),
        size_bytes: metadata.len(),
        permissions_octal,
    }))
}
