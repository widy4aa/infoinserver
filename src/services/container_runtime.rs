use serde::{Deserialize, Serialize};
use std::process::Command;

// ─────────────────────────────────────────────────────────────────────────────
// Runtime Detection
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuntimeKind {
    Podman,
    Docker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub kind: RuntimeKind,
    pub binary: String,
    pub version: String,
    pub compose_binary: String,
}

/// Auto-detect: coba podman dulu, fallback ke docker
pub fn detect_runtime() -> Result<RuntimeInfo, String> {
    // 1. Coba podman
    if let Ok(out) = Command::new("podman").arg("--version").output() {
        if out.status.success() {
            let version = String::from_utf8_lossy(&out.stdout)
                .split_whitespace()
                .nth(2)
                .unwrap_or("unknown")
                .trim()
                .to_string();

            let compose_binary = detect_compose_binary("podman");
            return Ok(RuntimeInfo {
                kind: RuntimeKind::Podman,
                binary: "podman".to_string(),
                version,
                compose_binary,
            });
        }
    }

    // 2. Fallback: coba docker
    if let Ok(out) = Command::new("docker").arg("--version").output() {
        if out.status.success() {
            let raw = String::from_utf8_lossy(&out.stdout).to_string();
            // "Docker version 24.x.x, build ..."
            let version = raw
                .split(',')
                .next()
                .unwrap_or(&raw)
                .split_whitespace()
                .last()
                .unwrap_or("unknown")
                .trim()
                .to_string();

            let compose_binary = detect_compose_binary("docker");
            return Ok(RuntimeInfo {
                kind: RuntimeKind::Docker,
                binary: "docker".to_string(),
                version,
                compose_binary,
            });
        }
    }

    Err("No container runtime found. Please install podman or docker.".to_string())
}

/// Pilih compose binary yang tersedia untuk runtime tertentu
fn detect_compose_binary(runtime: &str) -> String {
    // Cek `<runtime> compose` (v2 plugin)
    if let Ok(out) = Command::new(runtime)
        .args(["compose", "version"])
        .output()
    {
        if out.status.success() {
            return format!("{} compose", runtime);
        }
    }

    // Fallback: podman-compose / docker-compose (legacy)
    let legacy = if runtime == "podman" {
        "podman-compose"
    } else {
        "docker-compose"
    };
    if Command::new(legacy).arg("--version").output().is_ok() {
        return legacy.to_string();
    }

    // Jika tidak ada compose sama sekali, return empty string
    String::new()
}

// ─────────────────────────────────────────────────────────────────────────────
// Normalized Container struct (berlaku untuk podman & docker)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Container {
    pub id: String,
    pub short_id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub ports: Vec<String>,
    /// Label `com.docker.compose.project` jika ada
    pub compose_project: Option<String>,
    /// Label `com.docker.compose.service` jika ada
    pub compose_service: Option<String>,
    pub created: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Podman JSON structs (PascalCase)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct PodmanContainer {
    id: String,
    image: String,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    names: Option<Vec<String>>,
    #[serde(default)]
    ports: Option<serde_json::Value>,
    #[serde(default)]
    labels: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    created_at: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Docker JSON structs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct DockerContainer {
    #[serde(rename = "Id", default)]
    id: String,
    #[serde(rename = "Image", default)]
    image: String,
    #[serde(rename = "State", default)]
    state: String,
    #[serde(rename = "Status", default)]
    status: String,
    #[serde(rename = "Names", default)]
    names: Vec<String>,
    #[serde(rename = "Ports", default)]
    ports: serde_json::Value,
    #[serde(rename = "Labels", default)]
    labels: std::collections::HashMap<String, String>,
    #[serde(rename = "CreatedAt", default)]
    created_at: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Container operations
// ─────────────────────────────────────────────────────────────────────────────

pub fn list_containers(rt: &RuntimeInfo) -> Result<Vec<Container>, String> {
    let output = Command::new(&rt.binary)
        .args(["ps", "-a", "--format", "json"])
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", rt.binary, e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Runtime error: {}", err));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    if json_str.trim().is_empty() || json_str.trim() == "null" {
        return Ok(Vec::new());
    }

    match rt.kind {
        RuntimeKind::Podman => parse_podman_containers(&json_str),
        RuntimeKind::Docker => parse_docker_containers(&json_str),
    }
}

fn parse_podman_containers(json_str: &str) -> Result<Vec<Container>, String> {
    let raw: Vec<PodmanContainer> = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse podman JSON: {} — raw: {}", e, &json_str[..json_str.len().min(200)]))?;

    Ok(raw.into_iter().map(|c| {
        let labels = c.labels.unwrap_or_default();
        let compose_project = labels.get("com.docker.compose.project").cloned();
        let compose_service = labels.get("com.docker.compose.service").cloned();

        let ports = parse_podman_ports(&c.ports);
        let short_id = c.id[..c.id.len().min(12)].to_string();
        let name = c.names
            .and_then(|n| n.into_iter().next())
            .unwrap_or_else(|| short_id.clone());

        Container {
            short_id,
            id: c.id,
            name,
            image: c.image,
            state: c.state.unwrap_or_else(|| "unknown".to_string()),
            status: c.status.unwrap_or_default(),
            ports,
            compose_project,
            compose_service,
            created: c.created_at,
        }
    }).collect())
}

fn parse_docker_containers(json_str: &str) -> Result<Vec<Container>, String> {
    let raw: Vec<DockerContainer> = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse docker JSON: {}", e))?;

    Ok(raw.into_iter().map(|c| {
        let compose_project = c.labels.get("com.docker.compose.project").cloned();
        let compose_service = c.labels.get("com.docker.compose.service").cloned();

        // Docker ports is a string like "0.0.0.0:8080->80/tcp, ..."
        let ports = if let Some(arr) = c.ports.as_array() {
            arr.iter().filter_map(|p| {
                let host = p.get("HostPort").and_then(|v| v.as_str()).unwrap_or("");
                let cont = p.get("PrivatePort").and_then(|v| v.as_u64()).map(|v| v.to_string()).unwrap_or_default();
                if !host.is_empty() && !cont.is_empty() {
                    Some(format!("{}:{}", host, cont))
                } else if !cont.is_empty() {
                    Some(cont)
                } else {
                    None
                }
            }).collect()
        } else if let Some(s) = c.ports.as_str() {
            s.split(',').map(|p| p.trim().to_string()).filter(|s| !s.is_empty()).collect()
        } else {
            vec![]
        };

        let short_id = c.id[..c.id.len().min(12)].to_string();
        let name = c.names.into_iter().next()
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| short_id.clone());

        Container {
            short_id: short_id.clone(),
            id: c.id,
            name,
            image: c.image,
            state: c.state.to_lowercase(),
            status: c.status,
            ports,
            compose_project,
            compose_service,
            created: if c.created_at.is_empty() { None } else { Some(c.created_at) },
        }
    }).collect())
}

fn parse_podman_ports(ports: &Option<serde_json::Value>) -> Vec<String> {
    let Some(v) = ports else { return vec![] };
    if let Some(arr) = v.as_array() {
        arr.iter().filter_map(|p| {
            let host = p.get("host_port").and_then(|v| v.as_u64())?;
            let cont = p.get("container_port").and_then(|v| v.as_u64())?;
            Some(format!("{}:{}", host, cont))
        }).collect()
    } else {
        vec![]
    }
}

pub fn container_action(rt: &RuntimeInfo, action: &str, id: &str) -> Result<String, String> {
    // Validasi ID
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid container ID".to_string());
    }
    let valid_actions = ["start", "stop", "restart", "rm"];
    if !valid_actions.contains(&action) {
        return Err("Invalid action".to_string());
    }

    let mut args: Vec<&str> = vec![action];
    if action == "rm" {
        args.push("-f");
    }
    args.push(id);

    let out = Command::new(&rt.binary)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run {} {}: {}", rt.binary, action, e))?;

    if out.status.success() {
        Ok(format!("Container '{}' {} successfully", id, action))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

pub fn create_container(
    rt: &RuntimeInfo,
    name: &str,
    image: &str,
    ports: &[String],
    env_vars: &[String],
    volumes: &[String],
    restart_policy: Option<&str>,
) -> Result<String, String> {
    // Validasi
    if !is_valid_name_or_image(name) {
        return Err("Invalid container name".to_string());
    }
    if !is_valid_name_or_image(image) {
        return Err("Invalid image name".to_string());
    }
    for p in ports {
        if !is_valid_port_mapping(p) {
            return Err(format!("Invalid port mapping: {}", p));
        }
    }

    let mut args = vec!["run", "-d", "--name", name];

    for p in ports {
        args.push("-p");
        args.push(p.as_str());
    }
    for e in env_vars {
        args.push("-e");
        args.push(e.as_str());
    }
    for v in volumes {
        args.push("-v");
        args.push(v.as_str());
    }
    if let Some(policy) = restart_policy {
        args.push("--restart");
        args.push(policy);
    }
    args.push(image);

    let out = Command::new(&rt.binary)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to create container: {}", e))?;

    if out.status.success() {
        let container_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Ok(container_id)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

pub fn inspect_container(rt: &RuntimeInfo, id: &str) -> Result<serde_json::Value, String> {
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid container ID".to_string());
    }
    let out = Command::new(&rt.binary)
        .args(["inspect", id])
        .output()
        .map_err(|e| format!("Failed to inspect: {}", e))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
        .map_err(|e| format!("Failed to parse inspect JSON: {}", e))
}

pub fn container_logs(rt: &RuntimeInfo, id: &str, tail: u32) -> Result<String, String> {
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid container ID".to_string());
    }
    let tail_str = tail.to_string();
    let out = Command::new(&rt.binary)
        .args(["logs", "--tail", &tail_str, id])
        .output()
        .map_err(|e| format!("Failed to get logs: {}", e))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);
    if combined.trim().is_empty() {
        Ok("No logs available.".to_string())
    } else {
        Ok(combined)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn is_valid_name_or_image(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':' || c == '/' || c == '@'
    })
}

fn is_valid_port_mapping(s: &str) -> bool {
    // Accepts: "8080:80", "0.0.0.0:8080:80", "8080:80/tcp"
    let s = s.split('/').next().unwrap_or(s);
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => parts[0].parse::<u16>().is_ok() && parts[1].parse::<u16>().is_ok(),
        3 => parts[1].parse::<u16>().is_ok() && parts[2].parse::<u16>().is_ok(),
        _ => false,
    }
}
