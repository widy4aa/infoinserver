use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
    pub writable: bool, // true jika path berada di home atau removable mount
}

/// Validasi path agar tidak mengandung null bytes atau karakter berbahaya.
/// Tidak lagi memblokir akses ke luar FILE_ROOT — akses ke seluruh filesystem diizinkan.
/// Kembalikan canonical absolute path.
pub fn resolve_path_safe(requested_path: &str) -> Result<PathBuf, String> {
    if requested_path.contains('\0') {
        return Err("Path mengandung null byte".to_string());
    }

    let target = if requested_path.is_empty() || requested_path == "/" {
        PathBuf::from("/")
    } else {
        PathBuf::from(requested_path)
    };

    // Canonicalize untuk mendapatkan absolute path sungguhan
    let canonical = match target.canonicalize() {
        Ok(p) => p,
        Err(_) => return Err(format!("Path tidak ditemukan: {}", requested_path)),
    };

    Ok(canonical)
}

/// Fungsi lama untuk kompatibilitas — sekarang hanya memanggil resolve_path_safe
pub fn resolve_and_validate_path(_base_root: &str, requested_subpath: &str) -> Result<PathBuf, String> {
    // Jika subpath diawali dengan /, gunakan sebagai absolute path
    let path = if requested_subpath.starts_with('/') {
        requested_subpath.to_string()
    } else {
        // Legacy: gabungkan dengan base_root
        let root = Path::new(_base_root).canonicalize().map_err(|_| "Invalid root directory".to_string())?;
        format!("{}/{}", root.display(), requested_subpath)
    };

    resolve_path_safe(&path)
}

/// Tentukan apakah path bisa ditulis:
/// - Berada di dalam home_root (FILE_ROOT) → writable
/// - Berada di dalam salah satu removable mount → writable
/// - Semua lainnya → read-only
pub fn check_write_permission(path: &Path, home_root: &str, removable_mounts: &[String]) -> bool {
    // Cek apakah path ada di home
    if let Ok(home) = Path::new(home_root).canonicalize() {
        if path.starts_with(&home) {
            return true;
        }
    }

    // Cek apakah path ada di salah satu removable mount
    for mount in removable_mounts {
        if let Ok(mount_path) = Path::new(mount).canonicalize() {
            if path.starts_with(&mount_path) {
                return true;
            }
        }
    }

    false
}

/// Ambil daftar removable mount points dari /proc/mounts
pub fn get_removable_mounts() -> Vec<String> {
    let mut mounts = Vec::new();
    if let Ok(content) = fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let mountpoint = parts[1];
                // Removable biasanya di /media/ atau /mnt/
                if mountpoint.starts_with("/media/") || mountpoint.starts_with("/mnt/") {
                    mounts.push(mountpoint.to_string());
                }
            }
        }
    }
    mounts
}

pub fn list_directory(path: &PathBuf, home_root: &str) -> Result<Vec<FileInfo>, String> {
    let mut files = Vec::new();
    let removable_mounts = get_removable_mounts();

    let entries = fs::read_dir(path).map_err(|e| format!("Cannot read directory: {}", e))?;

    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata() {
            let modified = metadata
                .modified()
                .unwrap_or(UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let entry_path = entry.path();
            let writable = check_write_permission(&entry_path, home_root, &removable_mounts);

            files.push(FileInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified,
                writable,
            });
        }
    }

    // Sort: direktori di atas, lalu abjad
    files.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
    });

    Ok(files)
}
