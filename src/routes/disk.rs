use axum::{Json, http::StatusCode, extract::Extension};
use serde::{Deserialize, Serialize};
use std::process::Command;
use crate::auth::jwt_middleware::AuthUser;
use crate::routes::process_mgmt::sudo_exec;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockDevice {
    pub name: String,
    pub size: Option<String>,
    #[serde(rename = "type")]
    pub device_type: Option<String>,
    pub mountpoint: Option<String>,
    pub fstype: Option<String>,
    pub rm: Option<bool>,
    pub hotplug: Option<bool>,
    pub label: Option<String>,
    pub model: Option<String>,
    pub children: Option<Vec<BlockDevice>>,
    // Field tambahan (computed)
    #[serde(default)]
    pub used_percent: Option<u32>,
    #[serde(default)]
    pub used: Option<String>,
    #[serde(default)]
    pub available: Option<String>,
    #[serde(default)]
    pub is_removable: bool,
    #[serde(default)]
    pub mounted: bool,
}

#[derive(Deserialize)]
pub struct MountRequest {
    pub device: String, // e.g. "sda1"
    pub label: Option<String>,
}

#[derive(Deserialize)]
pub struct UmountRequest {
    pub device: String, // e.g. "sda1"
}

pub async fn get_disk_info_handler(
    Extension(_auth): Extension<AuthUser>,
) -> Result<Json<Vec<BlockDevice>>, (StatusCode, String)> {
    // Jalankan lsblk -J
    let lsblk_out = Command::new("lsblk")
        .args(["-J", "-o", "NAME,SIZE,TYPE,MOUNTPOINT,FSTYPE,RM,HOTPLUG,LABEL,MODEL"])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to run lsblk: {}", e)))?;

    if !lsblk_out.status.success() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "lsblk failed".to_string()));
    }

    let lsblk_str = String::from_utf8_lossy(&lsblk_out.stdout).to_string();

    // Parse JSON dari lsblk
    let lsblk_json: serde_json::Value = serde_json::from_str(&lsblk_str)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse lsblk output: {}", e)))?;

    let devices_raw = lsblk_json["blockdevices"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    // Parse ke struct
    let mut devices: Vec<BlockDevice> = serde_json::from_value(serde_json::Value::Array(devices_raw))
        .unwrap_or_default();

    // Ambil df info untuk usage
    let df_map = get_df_info();

    // Enrichment: set is_removable, mounted, used_percent
    enrich_devices(&mut devices, &df_map);

    // Filter: hanya tampilkan disk (type = disk) dan partisi yang relevan
    let result: Vec<BlockDevice> = devices.into_iter()
        .filter(|d| d.device_type.as_deref() != Some("loop"))
        .collect();

    Ok(Json(result))
}

fn get_df_info() -> std::collections::HashMap<String, (u32, String, String)> {
    // Returns: mountpoint -> (used_percent, used, available)
    let mut map = std::collections::HashMap::new();
    if let Ok(out) = Command::new("df").args(["-h", "--output=target,pcent,used,avail"]).output() {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let mountpoint = parts[0].to_string();
                    let pct_str = parts[1].trim_end_matches('%');
                    let used_pct = pct_str.parse::<u32>().unwrap_or(0);
                    let used = parts[2].to_string();
                    let avail = parts[3].to_string();
                    map.insert(mountpoint, (used_pct, used, avail));
                }
            }
        }
    }
    map
}

fn enrich_devices(devices: &mut Vec<BlockDevice>, df_map: &std::collections::HashMap<String, (u32, String, String)>) {
    for dev in devices.iter_mut() {
        let rm = dev.rm.unwrap_or(false);
        let hotplug = dev.hotplug.unwrap_or(false);
        dev.is_removable = rm || hotplug;

        if let Some(mp) = &dev.mountpoint {
            dev.mounted = !mp.is_empty() && mp != "[SWAP]";
            if let Some((pct, used, avail)) = df_map.get(mp) {
                dev.used_percent = Some(*pct);
                dev.used = Some(used.clone());
                dev.available = Some(avail.clone());
            }
        }

        // Rekursi untuk children
        if let Some(ref mut children) = dev.children {
            enrich_devices(children, df_map);
        }
    }
}

pub async fn mount_device_handler(
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<MountRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    // Validasi nama device (hanya alfanumerik dan slash)
    if !payload.device.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err((StatusCode::BAD_REQUEST, "Invalid device name".to_string()));
    }

    let device_path = format!("/dev/{}", payload.device);

    // Dapatkan label atau gunakan nama device
    let mount_label = payload.label.clone().unwrap_or_else(|| payload.device.clone());

    // Buat mount point di /media jika belum ada
    let mount_dir = format!("/media/{}", mount_label);
    let _ = sudo_exec(&password, &["mkdir", "-p", &mount_dir]);

    // Mount device
    let out = sudo_exec(&password, &["mount", &device_path, &mount_dir])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to mount: {}", e)))?;

    if out.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Device {} mounted at {}", device_path, mount_dir),
            "mountpoint": mount_dir
        })))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Mount failed: {}", err)))
    }
}

pub async fn umount_device_handler(
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<UmountRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let password = auth.0.pwd.clone();

    if !payload.device.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err((StatusCode::BAD_REQUEST, "Invalid device name".to_string()));
    }

    let device_path = format!("/dev/{}", payload.device);

    let out = sudo_exec(&password, &["umount", &device_path])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to umount: {}", e)))?;

    if out.status.success() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("Device {} unmounted successfully", device_path)
        })))
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Umount failed: {}", err)))
    }
}
