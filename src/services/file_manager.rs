use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Serialize, Deserialize, Debug)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
}

// Security: Validasi Path untuk mencegah Path Traversal
// Memastikan bahwa requested path selalu berakar/berada di dalam base_root
pub fn resolve_and_validate_path(base_root: &str, requested_subpath: &str) -> Result<PathBuf, String> {
    let root = Path::new(base_root).canonicalize().map_err(|_| "Invalid root directory in config".to_string())?;
    
    // Jangan izinkan karakter berbahaya
    if requested_subpath.contains("..") {
        return Err("Path traversal attempt blocked".to_string());
    }

    // Gabungkan path
    let mut target_path = root.clone();
    
    // Hilangkan leading slash agar join tidak mereset path
    let clean_subpath = requested_subpath.trim_start_matches('/');
    if !clean_subpath.is_empty() {
        target_path.push(clean_subpath);
    }

    // Canonicalize untuk mendapatkan absolute path sungguhan
    let canonical_target = match target_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return Err("Path not found".to_string()),
    };

    // Pengecekan krusial: pastikan path target dimulai dengan root path
    if !canonical_target.starts_with(&root) {
        return Err("Path traversal attempt blocked".to_string());
    }

    Ok(canonical_target)
}

pub fn list_directory(path: &PathBuf) -> Result<Vec<FileInfo>, String> {
    let mut files = Vec::new();

    let entries = fs::read_dir(path).map_err(|e| format!("Cannot read directory: {}", e))?;

    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata() {
            let modified = metadata
                .modified()
                .unwrap_or(UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            files.push(FileInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified,
            });
        }
    }

    // Sort: direktori di atas, lalu abjad
    files.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
    });

    Ok(files)
}
