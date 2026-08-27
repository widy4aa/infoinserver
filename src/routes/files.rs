use axum::{
    extract::Query,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::env;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use crate::services::file_manager::{list_directory, resolve_and_validate_path, FileInfo};

#[derive(Deserialize)]
pub struct FileQuery {
    pub path: Option<String>,
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
