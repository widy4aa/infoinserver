use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PodmanContainer {
    pub id: String,
    pub image: String,
    #[serde(default)]
    pub command: Option<Vec<String>>,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub names: Option<Vec<String>>,
    #[serde(default)]
    pub ports: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PodmanPort {
    pub host_port: Option<u16>,
    pub container_port: Option<u16>,
    pub host_ip: Option<String>,
}

pub fn list_containers() -> Result<Vec<PodmanContainer>, String> {
    let output = Command::new("podman")
        .args(["ps", "-a", "--format", "json"])
        .output()
        .map_err(|e| format!("Failed to execute podman: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Podman error: {}", err));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    
    // Jika tidak ada container, podman kadang return array kosong atau string kosong
    if json_str.trim().is_empty() {
        return Ok(Vec::new());
    }

    let containers: Vec<PodmanContainer> = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse podman JSON: {}", e))?;

    Ok(containers)
}

// Helper untuk command start/stop/restart/rm
// Memastikan id hanya alphanumeric untuk mencegah command injection
fn is_valid_id(id: &str) -> bool {
    id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn perform_container_action(action: &str, id: &str) -> Result<String, String> {
    if !is_valid_id(id) {
        return Err("Invalid container ID".to_string());
    }

    let valid_actions = ["start", "stop", "restart", "rm"];
    if !valid_actions.contains(&action) {
        return Err("Invalid action".to_string());
    }

    let mut args = vec![action, id];
    if action == "rm" {
        args.push("-f"); // force hapus
    }

    let output = Command::new("podman")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute podman {}: {}", action, e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Podman error: {}", err));
    }

    Ok(format!("Container {} {} successfully", id, action))
}