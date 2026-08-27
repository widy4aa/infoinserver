use axum::{
    extract::{Query, Multipart},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::env;
use std::process::Command;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use crate::services::file_manager::{list_directory, resolve_and_validate_path, FileInfo};

#[derive(Deserialize)]
pub struct FileQuery {
    pub path: Option<String>,
}

#[derive(Deserialize)]
pub struct FetchUrlRequest {
    pub url: String,
    pub path: String,
}

pub async fn list_files_handler(Query(query): Query<FileQuery>) -> Result<Json<Vec<FileInfo>>, (StatusCode, String)> {
    let base_root = env::var("FILE_ROOT").map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "FILE_ROOT not configured".to_string()))?;
    let req_path = query.path.unwrap_or_else(|| "/".to_string());

    match resolve_and_validate_path(&base_root, &req_path) {
        Ok(valid_path) => {
            match list_directory(&valid_path) {
                Ok(files) => Ok(Json(files)),
                Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
            }
        },
        Err(e) => Err((StatusCode::FORBIDDEN, e)),
    }
}

pub async fn download_file_handler(Query(query): Query<FileQuery>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let base_root = env::var("FILE_ROOT").map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "FILE_ROOT not configured".to_string()))?;
    let req_path = query.path.unwrap_or_default();

    if req_path.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Path parameter is required".to_string()));
    }

    let valid_path = resolve_and_validate_path(&base_root, &req_path)
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
    let base_root = env::var("FILE_ROOT").map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "FILE_ROOT not configured".to_string()))?;
    
    // Path tujuan upload (folder tujuan)
    let req_path = query.path.unwrap_or_else(|| "/".to_string());
    
    // Validasi folder tujuan
    let valid_dir = resolve_and_validate_path(&base_root, &req_path)
        .map_err(|e| (StatusCode::FORBIDDEN, e))?;

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
    let base_root = env::var("FILE_ROOT").map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "FILE_ROOT not configured".to_string()))?;
    
    let valid_dir = resolve_and_validate_path(&base_root, &payload.path)
        .map_err(|e| (StatusCode::FORBIDDEN, e))?;

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
