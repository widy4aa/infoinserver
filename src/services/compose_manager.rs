use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::PathBuf;
use std::io::Write;
use crate::services::container_runtime::RuntimeInfo;

const COMPOSE_DIR: &str = "/tmp/infoinserver-compose";

// ─────────────────────────────────────────────────────────────────────────────
// Data Structures
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComposeProject {
    pub name: String,
    pub directory: String,
    pub yaml_content: Option<String>,
    pub services: Vec<ComposeService>,
    pub status: String,          // "running" | "partial" | "stopped"
    pub source: String,          // "managed" (kita yang deploy) | "detected" (dari labels)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComposeService {
    pub name: String,
    pub container_id: Option<String>,
    pub image: String,
    pub state: String,
    pub status: String,
    pub ports: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Compose directory management
// ─────────────────────────────────────────────────────────────────────────────

fn project_dir(project_name: &str) -> PathBuf {
    PathBuf::from(COMPOSE_DIR).join(project_name)
}

fn compose_file_path(project_name: &str) -> PathBuf {
    project_dir(project_name).join("docker-compose.yml")
}

fn ensure_dir(project_name: &str) -> Result<(), String> {
    let dir = project_dir(project_name);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create project directory: {}", e))
}

fn is_valid_project_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

// ─────────────────────────────────────────────────────────────────────────────
// Compose command runner
// ─────────────────────────────────────────────────────────────────────────────

/// Build the command args for compose, handling "docker compose" vs "podman-compose" etc.
fn compose_cmd(rt: &RuntimeInfo, project_name: &str) -> Command {
    let compose_file = compose_file_path(project_name);
    let compose_file_str = compose_file.to_string_lossy().to_string();

    // "podman compose" or "docker compose" → split into binary + subcommand
    // "podman-compose" or "docker-compose" → direct binary
    if rt.compose_binary.contains(' ') {
        let parts: Vec<&str> = rt.compose_binary.splitn(2, ' ').collect();
        let mut cmd = Command::new(parts[0]);
        cmd.arg(parts[1]);
        cmd.args(["-f", &compose_file_str, "-p", project_name]);
        cmd
    } else {
        let mut cmd = Command::new(&rt.compose_binary);
        cmd.args(["-f", &compose_file_str, "-p", project_name]);
        cmd
    }
}

fn run_compose(rt: &RuntimeInfo, project_name: &str, extra_args: &[&str]) -> Result<String, String> {
    if rt.compose_binary.is_empty() {
        return Err("No compose tool found. Install podman-compose or docker-compose.".to_string());
    }

    let mut cmd = compose_cmd(rt, project_name);
    cmd.args(extra_args);

    let out = cmd.output()
        .map_err(|e| format!("Failed to run compose: {}", e))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if out.status.success() {
        Ok(format!("{}{}", stdout, stderr).trim().to_string())
    } else {
        Err(format!("{}{}", stderr, stdout).trim().to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Deploy compose project: simpan YAML, jalankan up -d --build
pub fn deploy_compose(rt: &RuntimeInfo, project_name: &str, yaml_content: &str) -> Result<String, String> {
    if !is_valid_project_name(project_name) {
        return Err("Invalid project name. Use only letters, numbers, dashes, underscores (max 64 chars).".to_string());
    }
    if yaml_content.trim().is_empty() {
        return Err("YAML content cannot be empty.".to_string());
    }

    ensure_dir(project_name)?;

    // Simpan YAML ke file
    let compose_file = compose_file_path(project_name);
    let mut f = std::fs::File::create(&compose_file)
        .map_err(|e| format!("Failed to write compose file: {}", e))?;
    f.write_all(yaml_content.as_bytes())
        .map_err(|e| format!("Failed to write YAML: {}", e))?;

    // Run up -d --build
    let result = run_compose(rt, project_name, &["up", "-d", "--build"])?;
    Ok(result)
}

/// Stop project: compose down
pub fn stop_compose(rt: &RuntimeInfo, project_name: &str) -> Result<String, String> {
    if !is_valid_project_name(project_name) {
        return Err("Invalid project name.".to_string());
    }
    // Jika ada file di managed dir, pakai itu
    if compose_file_path(project_name).exists() {
        return run_compose(rt, project_name, &["down"]);
    }
    // Fallback: pakai filter label
    stop_compose_by_label(rt, project_name)
}

fn stop_compose_by_label(rt: &RuntimeInfo, project_name: &str) -> Result<String, String> {
    // Hentikan semua container dengan label project
    let out = Command::new(&rt.binary)
        .args(["ps", "-q", "--filter", &format!("label=com.docker.compose.project={}", project_name)])
        .output()
        .map_err(|e| format!("Failed: {}", e))?;

    let ids = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if ids.is_empty() {
        return Ok("No running containers for this project.".to_string());
    }

    for id in ids.lines() {
        let _ = Command::new(&rt.binary).args(["stop", id]).output();
    }
    Ok(format!("Stopped project '{}'", project_name))
}

/// Restart project
pub fn restart_compose(rt: &RuntimeInfo, project_name: &str) -> Result<String, String> {
    if !is_valid_project_name(project_name) {
        return Err("Invalid project name.".to_string());
    }
    if compose_file_path(project_name).exists() {
        return run_compose(rt, project_name, &["restart"]);
    }
    Err("Compose file not found for this project. Only managed projects can be restarted here.".to_string())
}

/// Rebuild: force recreate
pub fn rebuild_compose(rt: &RuntimeInfo, project_name: &str) -> Result<String, String> {
    if !is_valid_project_name(project_name) {
        return Err("Invalid project name.".to_string());
    }
    if !compose_file_path(project_name).exists() {
        return Err("Compose file not found. Only managed projects can be rebuilt.".to_string());
    }
    run_compose(rt, project_name, &["up", "-d", "--build", "--force-recreate"])
}

/// Scale service
pub fn scale_service(rt: &RuntimeInfo, project_name: &str, service: &str, count: u32) -> Result<String, String> {
    if !is_valid_project_name(project_name) {
        return Err("Invalid project name.".to_string());
    }
    if !service.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid service name.".to_string());
    }
    if !compose_file_path(project_name).exists() {
        return Err("Compose file not found for managed scale.".to_string());
    }
    let scale_arg = format!("{}={}", service, count);
    run_compose(rt, project_name, &["up", "-d", "--scale", &scale_arg])
}

/// Get compose logs (all services or specific service)
pub fn compose_logs(rt: &RuntimeInfo, project_name: &str, service: Option<&str>, tail: u32) -> Result<String, String> {
    if !is_valid_project_name(project_name) {
        return Err("Invalid project name.".to_string());
    }
    let tail_str = tail.to_string();
    let mut args: Vec<&str> = vec!["logs", "--tail", &tail_str, "--no-color"];
    if let Some(svc) = service {
        if !svc.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err("Invalid service name.".to_string());
        }
        args.push(svc);
    }

    // Coba managed project dulu
    if compose_file_path(project_name).exists() {
        return run_compose(rt, project_name, &args);
    }

    // Fallback: ambil logs dari container dengan label
    get_logs_by_label(rt, project_name, service, tail)
}

fn get_logs_by_label(rt: &RuntimeInfo, project_name: &str, service: Option<&str>, tail: u32) -> Result<String, String> {
    let mut filter = format!("label=com.docker.compose.project={}", project_name);
    if let Some(svc) = service {
        filter = format!("label=com.docker.compose.service={}", svc);
    }

    let out = Command::new(&rt.binary)
        .args(["ps", "-q", "--filter", &filter])
        .output()
        .map_err(|e| format!("Failed: {}", e))?;

    let ids = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if ids.is_empty() {
        return Ok("No containers found for this project.".to_string());
    }

    let tail_str = tail.to_string();
    let mut all_logs = String::new();
    for id in ids.lines() {
        let log_out = Command::new(&rt.binary)
            .args(["logs", "--tail", &tail_str, id])
            .output();
        if let Ok(o) = log_out {
            all_logs.push_str(&String::from_utf8_lossy(&o.stdout));
            all_logs.push_str(&String::from_utf8_lossy(&o.stderr));
        }
    }
    Ok(if all_logs.is_empty() { "No logs available.".to_string() } else { all_logs })
}

/// Status per-service dari compose project
pub fn compose_ps(rt: &RuntimeInfo, project_name: &str) -> Result<Vec<ComposeService>, String> {
    // Query container list dan filter by label
    let out = Command::new(&rt.binary)
        .args(["ps", "-a", "--format", "json",
               "--filter", &format!("label=com.docker.compose.project={}", project_name)])
        .output()
        .map_err(|e| format!("Failed: {}", e))?;

    let json_str = String::from_utf8_lossy(&out.stdout).to_string();
    if json_str.trim().is_empty() || json_str.trim() == "null" {
        return Ok(vec![]);
    }

    // Parse menggunakan container runtime helper
    let containers = crate::services::container_runtime::list_containers(rt)?;
    let services: Vec<ComposeService> = containers
        .into_iter()
        .filter(|c| c.compose_project.as_deref() == Some(project_name))
        .map(|c| ComposeService {
            name: c.compose_service.unwrap_or_else(|| c.name.clone()),
            container_id: Some(c.id),
            image: c.image,
            state: c.state,
            status: c.status,
            ports: c.ports,
        })
        .collect();

    Ok(services)
}

/// List semua compose projects (managed + detected dari labels)
pub fn list_compose_projects(rt: &RuntimeInfo) -> Result<Vec<ComposeProject>, String> {
    // 1. Detect projects dari container labels
    let containers = crate::services::container_runtime::list_containers(rt)?;

    let mut projects: std::collections::HashMap<String, Vec<crate::services::container_runtime::Container>> = std::collections::HashMap::new();
    for c in containers {
        if let Some(ref project) = c.compose_project {
            projects.entry(project.clone()).or_default().push(c);
        }
    }

    let mut result: Vec<ComposeProject> = projects.into_iter().map(|(name, containers)| {
        let all_running = containers.iter().all(|c| c.state == "running");
        let any_running = containers.iter().any(|c| c.state == "running");
        let status = if all_running {
            "running".to_string()
        } else if any_running {
            "partial".to_string()
        } else {
            "stopped".to_string()
        };

        let services = containers.iter().map(|c| ComposeService {
            name: c.compose_service.clone().unwrap_or_else(|| c.name.clone()),
            container_id: Some(c.id.clone()),
            image: c.image.clone(),
            state: c.state.clone(),
            status: c.status.clone(),
            ports: c.ports.clone(),
        }).collect();

        // Cek apakah ini managed project (punya file compose di dir kita)
        let is_managed = compose_file_path(&name).exists();
        let yaml_content = if is_managed {
            std::fs::read_to_string(compose_file_path(&name)).ok()
        } else {
            None
        };

        ComposeProject {
            directory: project_dir(&name).to_string_lossy().to_string(),
            yaml_content,
            services,
            status,
            source: if is_managed { "managed".to_string() } else { "detected".to_string() },
            name,
        }
    }).collect();

    // 2. Tambahkan managed projects yang mungkin sedang stopped (tidak ada container aktif)
    if let Ok(entries) = std::fs::read_dir(COMPOSE_DIR) {
        for entry in entries.flatten() {
            let project_name = entry.file_name().to_string_lossy().to_string();
            if result.iter().any(|p| p.name == project_name) {
                continue; // Sudah terdeteksi dari containers
            }
            if compose_file_path(&project_name).exists() {
                let yaml_content = std::fs::read_to_string(compose_file_path(&project_name)).ok();
                result.push(ComposeProject {
                    name: project_name.clone(),
                    directory: project_dir(&project_name).to_string_lossy().to_string(),
                    yaml_content,
                    services: vec![],
                    status: "stopped".to_string(),
                    source: "managed".to_string(),
                });
            }
        }
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// Get YAML content dari managed project
pub fn get_compose_yaml(project_name: &str) -> Result<String, String> {
    if !is_valid_project_name(project_name) {
        return Err("Invalid project name.".to_string());
    }
    let path = compose_file_path(project_name);
    if !path.exists() {
        return Err(format!("Compose file not found for project '{}'", project_name));
    }
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read: {}", e))
}

/// Update YAML dan rebuild
pub fn update_compose_yaml(rt: &RuntimeInfo, project_name: &str, yaml_content: &str) -> Result<String, String> {
    if !is_valid_project_name(project_name) {
        return Err("Invalid project name.".to_string());
    }
    if yaml_content.trim().is_empty() {
        return Err("YAML content cannot be empty.".to_string());
    }

    ensure_dir(project_name)?;
    let compose_file = compose_file_path(project_name);
    let mut f = std::fs::File::create(&compose_file)
        .map_err(|e| format!("Failed to write compose file: {}", e))?;
    f.write_all(yaml_content.as_bytes())
        .map_err(|e| format!("Failed to write YAML: {}", e))?;

    // Apply changes
    run_compose(rt, project_name, &["up", "-d", "--build", "--force-recreate"])
}

/// Hapus managed project (down + hapus file)
pub fn delete_compose_project(rt: &RuntimeInfo, project_name: &str, remove_volumes: bool) -> Result<String, String> {
    if !is_valid_project_name(project_name) {
        return Err("Invalid project name.".to_string());
    }

    let mut args = vec!["down"];
    if remove_volumes {
        args.push("-v");
    }

    // Stop dulu kalau managed
    if compose_file_path(project_name).exists() {
        let _ = run_compose(rt, project_name, &args);
        // Hapus direktori
        let _ = std::fs::remove_dir_all(project_dir(project_name));
        return Ok(format!("Project '{}' removed.", project_name));
    }

    // Fallback: stop by label
    stop_compose_by_label(rt, project_name)?;
    Ok(format!("Project '{}' stopped (not a managed project, files not removed).", project_name))
}
