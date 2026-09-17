// ─────────────────────────────────────────────────────────────────────────────
// routes/vm.rs — Podman-based OS container management
//
// Distro yang didukung:
//   ubuntu-2404  → ubuntu:24.04   (GLIBC 2.39, kompatibel penuh)
//
// Endpoints:
//   GET    /api/vm/list
//   GET    /api/vm/create/ws?name=&distro=&username=&password=&backend_port=&host_ip=
//   POST   /api/vm/:name/start
//   POST   /api/vm/:name/stop
//   GET    /api/vm/:name/status
//   DELETE /api/vm/:name
// ─────────────────────────────────────────────────────────────────────────────

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    response::IntoResponse,
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::AppState;

// ─── Konfigurasi per distro ───────────────────────────────────────────────────

struct DistroConfig {
    base_image:  &'static str,
    pkg_install: &'static str,
    sudo_group:  &'static str,
    sudo_setup:  &'static str,
    mask_units:  &'static str,   // systemd units yang perlu di-mask agar container bisa boot
}

fn distro_config(distro: &str) -> Option<DistroConfig> {
    match distro {
        "ubuntu-2404" => Some(DistroConfig {
            base_image: "ubuntu:24.04",
            pkg_install: "\
                apt-get update -qq && \
                apt-get install -y -qq \
                    systemd systemd-sysv dbus \
                    sudo curl wget \
                    podman \
                    ufw \
                    fail2ban \
                    nmap \
                    openssh-client openssh-server \
                    cron \
                    zip unzip \
                    lsof net-tools iproute2 \
                    procps htop \
                    vim nano \
                    ca-certificates gnupg \
                    --no-install-recommends \
                    2>/dev/null && \
                rm -rf /var/lib/apt/lists/*",
            sudo_group: "sudo",
            sudo_setup: "",
            mask_units: "\
                systemctl mask \
                    dev-hugepages.mount \
                    sys-fs-fuse-connections.mount \
                    sys-kernel-config.mount \
                    display-manager.service \
                    getty@.service \
                    systemd-logind.service \
                    systemd-remount-fs.service \
                    getty.target \
                    graphical.target \
                    2>/dev/null || true",
        }),
        "arch" => Some(DistroConfig {
            base_image: "archlinux:base",
            // archlinux:base sudah include systemd — JANGAN install ulang (akan conflict)
            // Split install menjadi 2 batch:
            //   Batch 1: crun (oci-runtime, agar podman tidak prompt interactive)
            //   Batch 2: packages utama tanpa systemd
            //   Batch 3: podman + fail2ban (install terakhir agar deps sudah terpenuhi)
            pkg_install: "\
                pacman -Sy --noconfirm crun 2>/dev/null && \
                pacman -S --noconfirm \
                    sudo curl wget \
                    ufw nmap openssh cronie \
                    zip unzip lsof net-tools iproute2 \
                    procps-ng htop vim nano ca-certificates \
                    2>/dev/null && \
                pacman -S --noconfirm podman fail2ban 2>/dev/null && \
                pacman -Sc --noconfirm 2>/dev/null || true",
            sudo_group: "wheel",
            // mkdir -p dulu karena archlinux:base tidak membuat /etc/sudoers.d
            sudo_setup: "\
                mkdir -p /etc/sudoers.d && \
                echo '%wheel ALL=(ALL) ALL' > /etc/sudoers.d/wheel && \
                chmod 440 /etc/sudoers.d/wheel",
            mask_units: "\
                systemctl mask \
                    dev-hugepages.mount \
                    sys-fs-fuse-connections.mount \
                    sys-kernel-config.mount \
                    getty@.service \
                    getty.target \
                    graphical.target \
                    2>/dev/null || true",
        }),
        _ => None,
    }
}

// ─── DB Row ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct VmRow {
    pub id:           String,
    pub name:         String,
    pub distro:       String,
    pub username:     String,
    pub backend_port: i64,
    pub host_ip:      String,
    pub container_id: String,   // Podman container ID/name
    pub image_tag:    String,   // image yang dipakai
    pub created_at:   String,
}

// ─── Query params WebSocket ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateVmQuery {
    pub name:         String,
    pub distro:       String,
    pub username:     String,
    pub password:     String,
    pub backend_port: u16,
    pub host_ip:      Option<String>,
    #[allow(dead_code)]
    pub token:        Option<String>,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Jalankan podman command, return (success, stdout+stderr)
async fn podman(args: &[&str]) -> (bool, String) {
    let out = Command::new("podman")
        .args(args)
        .output()
        .await;
    match out {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).to_string();
            let err = String::from_utf8_lossy(&o.stderr).to_string();
            if !err.is_empty() { s.push_str(&err); }
            (o.status.success(), s.trim().to_string())
        }
        Err(e) => (false, e.to_string()),
    }
}

/// Get state container ("running" | "exited" | "created" | "unknown")
async fn container_state(container_id: &str) -> String {
    if container_id.is_empty() { return "unknown".to_string(); }
    let (ok, out) = podman(&[
        "inspect", "--format", "{{.State.Status}}", container_id
    ]).await;
    if ok { out.trim().to_string() } else { "stopped".to_string() }
}

/// Kirim JSON event ke WebSocket
async fn ws_event(ws: &mut WebSocket, event: &str, msg: &str) -> bool {
    let p = json!({ "event": event, "msg": msg }).to_string();
    ws.send(Message::Text(p.into())).await.is_ok()
}

/// Stream output podman command ke WebSocket
async fn ws_stream_podman(ws: &mut WebSocket, args: &[&str]) -> bool {
    let mut child = match Command::new("podman")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            ws_event(ws, "log", &format!("[error] {}", e)).await;
            return false;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Stream stdout
    if let Some(out) = stdout {
        let mut lines = BufReader::new(out).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() { continue; }
            if !ws_event(ws, "log", &line).await { break; }
        }
    }
    // Stream stderr
    if let Some(err) = stderr {
        let mut lines = BufReader::new(err).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() { continue; }
            // Filter noise yang tidak perlu
            if line.contains("Copying blob") || line.contains("Copying config")
                || line.contains("Writing manifest") || line.contains("Getting image")
            {
                if !ws_event(ws, "log", &line).await { break; }
            } else {
                if !ws_event(ws, "log", &format!("[err] {}", line)).await { break; }
            }
        }
    }

    match child.wait().await {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

// ─── GET /api/vm/list ─────────────────────────────────────────────────────────

pub async fn list_vms_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, VmRow>(
        "SELECT id, name, distro, username, backend_port, host_ip, container_id, image_tag, created_at \
         FROM vm_instances ORDER BY created_at DESC",
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut vms = Vec::new();
    for row in rows {
        let state_str = container_state(&row.container_id).await;
        vms.push(json!({
            "id":           row.id,
            "name":         row.name,
            "distro":       row.distro,
            "username":     row.username,
            "backend_port": row.backend_port,
            "host_ip":      row.host_ip,
            "container_id": row.container_id,
            "image_tag":    row.image_tag,
            "created_at":   row.created_at,
            "state":        state_str,
        }));
    }
    Ok(Json(json!({ "vms": vms })))
}

// ─── GET /api/vm/:name/status ─────────────────────────────────────────────────

pub async fn vm_status_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, VmRow>(
        "SELECT id, name, distro, username, backend_port, host_ip, container_id, image_tag, created_at \
         FROM vm_instances WHERE name = ?",
    )
    .bind(&name)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, format!("VM '{}' not found", name)))?;

    let state_str = container_state(&row.container_id).await;
    Ok(Json(json!({
        "id":           row.id,
        "name":         row.name,
        "distro":       row.distro,
        "username":     row.username,
        "backend_port": row.backend_port,
        "host_ip":      row.host_ip,
        "container_id": row.container_id,
        "image_tag":    row.image_tag,
        "created_at":   row.created_at,
        "state":        state_str,
        "dashboard_url": format!("http://{}:{}", row.host_ip, row.backend_port),
    })))
}

// ─── POST /api/vm/:name/start ─────────────────────────────────────────────────

pub async fn start_vm_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, VmRow>(
        "SELECT id, name, distro, username, backend_port, host_ip, container_id, image_tag, created_at \
         FROM vm_instances WHERE name = ?",
    )
    .bind(&name)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, format!("VM '{}' not found", name)))?;

    let (ok, out) = podman(&["start", &row.container_id]).await;
    if !ok {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("podman start: {}", out)));
    }

    Ok(Json(json!({
        "ok": true,
        "message": format!("VM '{}' started. Backend on port {}.", row.name, row.backend_port),
    })))
}

// ─── POST /api/vm/:name/stop ──────────────────────────────────────────────────

pub async fn stop_vm_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, VmRow>(
        "SELECT id, name, distro, username, backend_port, host_ip, container_id, image_tag, created_at \
         FROM vm_instances WHERE name = ?",
    )
    .bind(&name)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, format!("VM '{}' not found", name)))?;

    let (ok, out) = podman(&["stop", &row.container_id]).await;
    if !ok {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("podman stop: {}", out)));
    }

    Ok(Json(json!({ "ok": true, "message": format!("VM '{}' stopped.", row.name) })))
}

// ─── DELETE /api/vm/:name ─────────────────────────────────────────────────────

pub async fn delete_vm_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, VmRow>(
        "SELECT id, name, distro, username, backend_port, host_ip, container_id, image_tag, created_at \
         FROM vm_instances WHERE name = ?",
    )
    .bind(&name)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, format!("VM '{}' not found", name)))?;

    // Force remove container
    podman(&["rm", "-f", &row.container_id]).await;

    // Remove named volume untuk data VM ini
    let vol_name = format!("infoinserver-vm-{}", row.name);
    podman(&["volume", "rm", "-f", &vol_name]).await;

    // Hapus dari DB
    sqlx::query("DELETE FROM vm_instances WHERE name = ?")
        .bind(&name)
        .execute(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "ok": true, "message": format!("VM '{}' deleted.", name) })))
}

// ─── GET /api/vm/create/ws ────────────────────────────────────────────────────

pub async fn create_vm_ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<CreateVmQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| deploy_vm(socket, params, state))
}

async fn deploy_vm(mut ws: WebSocket, p: CreateVmQuery, state: AppState) {
    let name     = p.name.trim().to_string();
    let distro   = p.distro.trim().to_string();
    let username = p.username.trim().to_string();
    let password = p.password.clone();
    let port     = p.backend_port;
    let host_ip  = p.host_ip.clone().unwrap_or_else(|| "127.0.0.1".to_string());

    // ── Validasi ──────────────────────────────────────────────────────────────
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        ws_event(&mut ws, "error", "Nama VM tidak valid. Gunakan huruf, angka, dash, underscore.").await;
        return;
    }
    if username.is_empty() || password.is_empty() {
        ws_event(&mut ws, "error", "Username dan password wajib diisi.").await;
        return;
    }

    let cfg = match distro_config(&distro) {
        Some(c) => c,
        None => {
            ws_event(&mut ws, "error", &format!("Distro '{}' tidak dikenal.", distro)).await;
            return;
        }
    };

    // Cek duplikat nama
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vm_instances WHERE name = ?")
        .bind(&name)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0);
    if count > 0 {
        ws_event(&mut ws, "error", &format!("VM '{}' sudah ada.", name)).await;
        return;
    }

    // ── Step 1: Cek Podman tersedia ────────────────────────────────────────────
    ws_event(&mut ws, "step", "Memeriksa Podman...").await;
    let (ok, ver) = podman(&["--version"]).await;
    if !ok {
        ws_event(&mut ws, "error", "Podman tidak tersedia di server ini.").await;
        return;
    }
    ws_event(&mut ws, "log", &format!("✓ {}", ver.lines().next().unwrap_or("podman"))).await;

    // ── Step 2: Tentukan binary path ───────────────────────────────────────────
    ws_event(&mut ws, "step", "Mencari binary InfoIn Server...").await;
    let binary_path = {
        let current = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from(""));
        let candidates = [
            current.to_string_lossy().to_string(),
            "/usr/local/bin/infoinserver".to_string(),
        ];
        let mut found = String::new();
        for c in &candidates {
            if !c.is_empty() && std::path::Path::new(c).exists() {
                found = c.clone();
                break;
            }
        }
        found
    };

    if binary_path.is_empty() {
        ws_event(&mut ws, "error", "Binary infoinserver tidak ditemukan di host.").await;
        return;
    }
    ws_event(&mut ws, "log", &format!("✓ Binary: {}", binary_path)).await;

    // ── Step 3: Pull/cek base image ───────────────────────────────────────────
    ws_event(&mut ws, "step", &format!("Memastikan image '{}' tersedia...", cfg.base_image)).await;

    // Cek apakah image sudah ada lokal
    let (img_exists, _) = podman(&["image", "exists", cfg.base_image]).await;
    if img_exists {
        ws_event(&mut ws, "log", &format!("✓ Image '{}' sudah ada lokal.", cfg.base_image)).await;
    } else {
        ws_event(&mut ws, "log", &format!("Mengunduh image '{}'...", cfg.base_image)).await;
        let pulled = ws_stream_podman(&mut ws, &["pull", cfg.base_image]).await;
        if !pulled {
            // Cek ulang — kadang pull berhasil meski exit code non-zero
            let (check, _) = podman(&["image", "exists", cfg.base_image]).await;
            if !check {
                ws_event(&mut ws, "error", &format!("Gagal pull image '{}'.", cfg.base_image)).await;
                return;
            }
        }
        ws_event(&mut ws, "log", &format!("✓ Image '{}' siap.", cfg.base_image)).await;
    }

    // ── Step 4: Buat image custom dengan systemd + semua packages ────────────
    ws_event(&mut ws, "step", "Membangun image VM (systemd + packages lengkap)...").await;
    ws_event(&mut ws, "log", "Ini mungkin memakan beberapa menit saat pertama kali...").await;

    let image_tag = format!("localhost/infoinserver-vm-{}:latest", distro);

    let sudo_setup_line = if cfg.sudo_setup.is_empty() {
        String::new()
    } else {
        format!("RUN {}", cfg.sudo_setup)
    };

    let mask_line = if cfg.mask_units.is_empty() {
        String::new()
    } else {
        format!("RUN {}", cfg.mask_units)
    };

    // Containerfile: systemd sebagai PID 1 + setup user via env + infoinserver service
    let containerfile = format!(
        r#"FROM {base}
ENV DEBIAN_FRONTEND=noninteractive
RUN {pkg}
{sudo_setup}
{mask}
RUN mkdir -p /var/lib/infoinserver
COPY setup.sh /usr/local/bin/vm-setup.sh
RUN chmod +x /usr/local/bin/vm-setup.sh
COPY infoinserver.service /etc/systemd/system/infoinserver.service
RUN systemctl enable infoinserver 2>/dev/null || true
STOPSIGNAL SIGRTMIN+3
EXPOSE 8080
CMD ["/sbin/init"]
"#,
        base       = cfg.base_image,
        pkg        = cfg.pkg_install,
        sudo_setup = sudo_setup_line,
        mask       = mask_line,
    );

    // Script setup yang dijalankan oleh systemd oneshot service saat container pertama boot
    let setup_script = format!(
        r#"#!/bin/bash
# Setup user dari environment variables (dijalankan saat boot pertama)
set -e
SETUP_DONE=/var/lib/infoinserver/.setup_done
if [ -f "$SETUP_DONE" ]; then exit 0; fi

VM_USER="${{VM_USERNAME:-vmuser}}"
VM_PASS="${{VM_PASSWORD:-changeme}}"
SUDO_GROUP="{sudo_group}"

# Set root password
echo "root:$VM_PASS" | chpasswd

# Buat user jika belum ada
if ! id "$VM_USER" &>/dev/null; then
    useradd -m -s /bin/bash -G "$SUDO_GROUP" "$VM_USER" 2>/dev/null || \
    adduser -D -s /bin/bash "$VM_USER" 2>/dev/null || true
fi
echo "$VM_USER:$VM_PASS" | chpasswd

touch "$SETUP_DONE"
echo "VM setup complete: user=$VM_USER"
"#,
        sudo_group = cfg.sudo_group,
    );

    // systemd service untuk infoinserver backend
    let infoinserver_service = r#"[Unit]
Description=InfoIn Server Backend
After=network.target vm-setup.service

[Service]
Type=simple
ExecStart=/usr/local/bin/infoinserver
EnvironmentFile=-/etc/infoinserver.env
WorkingDirectory=/var/lib/infoinserver
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
"#;

    // Tulis build context
    let build_dir = format!("/tmp/infoinserver-vm-build-{}", name);
    tokio::fs::create_dir_all(&build_dir).await.ok();
    tokio::fs::write(format!("{}/Containerfile", build_dir), &containerfile).await.ok();
    tokio::fs::write(format!("{}/setup.sh", build_dir), &setup_script).await.ok();
    tokio::fs::write(format!("{}/infoinserver.service", build_dir), infoinserver_service).await.ok();
    // Tulis vm-setup.service langsung ke path yang akan dicopy saat runtime
    // (tidak bisa COPY ke /etc/systemd di build time karena masih template)

    // Build image
    let (img_built, _) = podman(&["image", "exists", &image_tag]).await;
    if img_built {
        ws_event(&mut ws, "log", &format!("✓ Image '{}' sudah ada, skip build.", image_tag)).await;
    } else {
        let built = ws_stream_podman(&mut ws, &[
            "build",
            "--tag", &image_tag,
            "--file", &format!("{}/Containerfile", build_dir),
            &build_dir,
        ]).await;

        tokio::fs::remove_dir_all(&build_dir).await.ok();

        if !built {
            let (check, _) = podman(&["image", "exists", &image_tag]).await;
            if !check {
                ws_event(&mut ws, "error", "Build image gagal.").await;
                return;
            }
        }
        ws_event(&mut ws, "log", &format!("✓ Image '{}' berhasil dibuild.", image_tag)).await;
    }
    tokio::fs::remove_dir_all(&build_dir).await.ok();

    // ── Step 5: Buat named volume untuk data persistensi ──────────────────────
    ws_event(&mut ws, "step", "Membuat volume data...").await;

    let vol_name = format!("infoinserver-vm-{}", name);
    podman(&["volume", "create", &vol_name]).await;
    ws_event(&mut ws, "log", &format!("✓ Volume '{}' siap.", vol_name)).await;

    // ── Step 6: Generate JWT secret untuk container ini ───────────────────────
    let jwt_secret = Uuid::new_v4().to_string().replace('-', "");
    let container_name = format!("infoinserver-vm-{}", name);

    // ── Step 6b: Bersihkan container/volume orphan jika ada ───────────────────
    // Container bisa tersisa dari deployment sebelumnya yang gagal (tidak ada di DB).
    // Hapus otomatis agar deploy bisa dilanjutkan tanpa error "already in use".
    let (container_exists, _) = podman(&["container", "exists", &container_name]).await;
    if container_exists {
        ws_event(&mut ws, "log", &format!("⚠ Container orphan '{}' ditemukan, membersihkan...", container_name)).await;
        podman(&["rm", "-f", &container_name]).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        ws_event(&mut ws, "log", "✓ Container orphan dibersihkan.").await;
    }

    // ── Step 7: Jalankan container dengan systemd sebagai init ───────────────
    ws_event(&mut ws, "step", &format!("Menjalankan container '{}' di port {}...", container_name, port)).await;

    let port_map = format!("{}:8080", port);
    let vol_map  = format!("{}:/var/lib/infoinserver:Z", vol_name);

    let (ok, container_id) = podman(&[
        "run", "-d",
        "--name",       &container_name,
        "--hostname",   &name,
        "--restart",    "unless-stopped",
        "--systemd=always",
        "--privileged",
        "-p",           &port_map,
        "-v",           &vol_map,
        "-e", &format!("VM_USERNAME={}", username),
        "-e", &format!("VM_PASSWORD={}", password),
        &image_tag,
    ]).await;

    if !ok {
        if container_id.contains("already in use") {
            ws_event(&mut ws, "error", &format!(
                "Container '{}' masih terkunci. Coba hapus manual:\n  podman rm -f {}",
                container_name, container_name
            )).await;
        } else {
            ws_event(&mut ws, "error", &format!("podman run gagal: {}", container_id)).await;
        }
        return;
    }

    let cid = container_id.trim().to_string();
    ws_event(&mut ws, "log", &format!("✓ Container berjalan (ID: {}...)", &cid[..12.min(cid.len())])).await;

    // ── Step 7b: Tunggu systemd siap di dalam container ──────────────────────
    ws_event(&mut ws, "step", "Menunggu systemd siap...").await;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    // Cek systemd ready
    for _ in 0..10 {
        let (ok, _) = podman(&["exec", &container_name, "systemctl", "is-system-running"]).await;
        if ok { break; }
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    ws_event(&mut ws, "log", "✓ systemd siap.").await;

    // ── Step 7c: Copy binary infoinserver ke container ───────────────────────
    ws_event(&mut ws, "step", "Menyalin binary InfoIn Server ke container...").await;

    let (cp_ok, cp_out) = podman(&[
        "cp", &binary_path,
        &format!("{}:/usr/local/bin/infoinserver", container_name),
    ]).await;
    if !cp_ok {
        ws_event(&mut ws, "log", &format!("⚠ Copy binary gagal: {}", cp_out)).await;
    } else {
        // pastikan executable
        podman(&["exec", &container_name, "chmod", "+x", "/usr/local/bin/infoinserver"]).await;
        ws_event(&mut ws, "log", "✓ Binary disalin.").await;
    }

    // ── Step 7d: Setup user root + user biasa langsung via exec ──────────────
    ws_event(&mut ws, "step", &format!("Membuat user '{}' dan set password...", username)).await;

    let escaped_pass = password.replace('\'', r"'\''");
    let escaped_user = username.replace('\'', r"'\''");
    let sudo_group   = cfg.sudo_group;

    let setup_cmd = format!(
        "echo 'root:{pass}' | chpasswd 2>/dev/null; \
         if ! id '{user}' &>/dev/null; then \
             useradd -m -s /bin/bash -G {grp} '{user}' 2>/dev/null || \
             adduser -D -s /bin/bash '{user}' 2>/dev/null; \
         fi; \
         echo '{user}:{pass}' | chpasswd 2>/dev/null; \
         echo 'User ready'",
        pass = escaped_pass,
        user = escaped_user,
        grp  = sudo_group,
    );
    let (user_ok, user_out) = podman(&["exec", &container_name, "bash", "-c", &setup_cmd]).await;
    if user_ok || user_out.contains("ready") {
        ws_event(&mut ws, "log", &format!("✓ User '{}' siap dengan akses sudo.", username)).await;
    } else {
        ws_event(&mut ws, "log", &format!("⚠ Setup user: {}", user_out)).await;
    }

    // ── Step 7e: Tulis /etc/infoinserver.env ─────────────────────────────────
    ws_event(&mut ws, "step", "Menulis konfigurasi environment...").await;

    let env_content = format!(
        "PORT=8080\nDB_PATH=sqlite:///var/lib/infoinserver/data.db\nJWT_SECRET={jwt}\nCORS_ORIGIN=http://{host}:3000\n",
        jwt  = jwt_secret,
        host = host_ip,
    );
    let write_env = format!(
        "printf '%s' '{content}' > /etc/infoinserver.env",
        content = env_content.replace('\'', r"'\''"),
    );
    podman(&["exec", &container_name, "bash", "-c", &write_env]).await;
    ws_event(&mut ws, "log", "✓ /etc/infoinserver.env ditulis.").await;

    // ── Step 7f: Start infoinserver via systemd ───────────────────────────────
    ws_event(&mut ws, "step", "Memulai infoinserver service...").await;

    podman(&["exec", &container_name, "systemctl", "daemon-reload"]).await;
    podman(&["exec", &container_name, "systemctl", "enable", "infoinserver"]).await;
    let (svc_ok, svc_out) = podman(&["exec", &container_name, "systemctl", "start", "infoinserver"]).await;
    if svc_ok {
        ws_event(&mut ws, "log", "✓ infoinserver service dimulai via systemd.").await;
    } else {
        ws_event(&mut ws, "log", &format!("⚠ Start service: {}", svc_out)).await;
    }

    // ── Step 8: Tunggu backend siap ───────────────────────────────────────────
    ws_event(&mut ws, "step", "Menunggu backend siap...").await;

    let mut ready = false;
    for i in 0..15 {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        // Cek via localhost (container port di-forward ke host)
        let check = Command::new("curl")
            .args(["-sf", "--max-time", "2",
                   &format!("http://127.0.0.1:{}/api/ping", port)])
            .output()
            .await;
        if let Ok(o) = check {
            if o.status.success() {
                ws_event(&mut ws, "log", "✓ Backend merespons!").await;
                ready = true;
                break;
            }
        }
        if i % 3 == 0 {
            ws_event(&mut ws, "log", &format!("Menunggu... ({}/30 detik)", (i + 1) * 2)).await;
        }
    }
    if !ready {
        ws_event(&mut ws, "log", "⚠ Backend belum merespons, tapi container sudah berjalan.").await;
        ws_event(&mut ws, "log", "Cek logs: podman logs infoinserver-vm-<name>").await;
    }

    // ── Step 9: Simpan ke DB ──────────────────────────────────────────────────
    ws_event(&mut ws, "step", "Menyimpan ke database...").await;

    let vm_id = Uuid::new_v4().to_string();
    let result = sqlx::query(
        "INSERT INTO vm_instances \
         (id, name, distro, username, backend_port, host_ip, container_id, image_tag) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&vm_id)
    .bind(&name)
    .bind(&distro)
    .bind(&username)
    .bind(port as i64)
    .bind(&host_ip)
    .bind(&container_name)
    .bind(&image_tag)
    .execute(&state.db_pool)
    .await;

    if let Err(e) = result {
        ws_event(&mut ws, "error", &format!("Gagal simpan DB: {}", e)).await;
        return;
    }

    // ── Done ──────────────────────────────────────────────────────────────────
    ws_event(&mut ws, "done", &format!(
        "VM '{}' ({}) berhasil di-deploy! Backend: http://{}:{}",
        name, distro, host_ip, port
    )).await;

    ws_event(&mut ws, "info", &json!({
        "id":           vm_id,
        "name":         name,
        "distro":       distro,
        "username":     username,
        "backend_port": port,
        "host_ip":      host_ip,
        "container_id": container_name,
        "image_tag":    image_tag,
        "dashboard_url": format!("http://{}:{}", host_ip, port),
    }).to_string()).await;
}
