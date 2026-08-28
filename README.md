# Server Monitoring Dashboard

A lightweight, self-hosted server monitoring & administration dashboard. Powered by a **Rust backend** (Axum + Tokio) and a **Vue.js frontend**. Built specifically for Linux bare-metal servers — it reads metrics directly from the kernel (`/proc`, `/sys`) just like Node Exporter, streams real-time data via WebSocket, and is secured natively with PAM-based OS authentication and JWT session tokens.

---

## Features

### System Monitoring
- **Real-time Metrics via WebSocket** — CPU usage, RAM, Uptime, OS, and Kernel info streamed live without polling overhead.
- **Historical Performance Charts** — CPU, RAM, Disk, and Network (Mbps) usage recorded every 5 minutes to SQLite, displayed as interactive line charts with a customizable time range (24h, 12h, 6h, 3h, 1h).
- **Storage (Disks)** — Per-mount-point disk usage with progress bars.

### System Administration
- **Top Processes** — Live process list sortable by CPU or RAM, with keyword/PID search and force-kill support.
- **Systemd Services** — Full UI to manage background services/daemons. Start, Stop, Restart, Enable (auto-start), Disable, and search through loaded units.
- **System Journal Viewer** — A native-looking console for `journalctl` logs. Features syntax highlighting (colors for hostname, service, errors/warnings) and filtering by Auth (SSH) or Kernel (dmesg).
- **Cronjob Manager** — Read, add, edit, and delete scheduled tasks for the `root` user (`/etc/crontab`) via a clean UI.
- **Users & Groups** — Create Linux users, change passwords, assign secondary groups, and delete users (including home directories). Filter out system users dynamically.

### File Management
- **Advanced File Explorer** — Sandboxed web file browser (scoped to `FILE_ROOT`). 
- **Context Menu Actions (Right-Click)**:
  - Copy, Move, Rename, Delete files/folders.
  - **Compress to Zip** & **Extract Zip** (with password support).
  - **Permissions (chmod)**: Change file access rights instantly (e.g., 0755, 0644).
  - **Open as Text**: A built-in code editor to view and edit text files directly from the browser.
- **Remote Fetch**: Download files from the internet directly to the server via `wget`.

### Network & Security
- **Network Interfaces** — Active interfaces, MAC addresses, and RX/TX traffic.
- **Listening Ports** — All locally listening ports and their associated processes via `ss`, with kill-process action.
- **Deep Port Scan** — On-demand async port scanning via `nmap` with a background job queue.
- **UFW Firewall** — View firewall status, toggle enable/disable, and manage allow/deny rules.

### Container & Network Tools
- **Podman Integration** — List containers, view status, Start / Stop / Restart / Delete, and deploy new containers from the UI.
- **Cloudflare Integration** — Manage Cloudflare tunnels (Quick Tunnel & Managed Tunnel with token) and Zero Trust ingress routes via API.
- **Speedtest** — Scheduled (hourly) and on-demand network speed testing via `speedtest-cli`.

### Developer Tools & Alerts
- **Multi-Tab Root Terminal** — Native PTY shell via WebSocket (`xterm.js`). Supports multiple tabs, each with its own independent shell session that runs safely in the background.
- **Activity Logs & Alerts** — Background scheduler detects anomalies (CPU > 90%, RAM > 90%, Disk free space < 10%) and writes to an `activity_log` table with severity levels (INFO / WARNING / CRITICAL).

### Security Architecture
- **PAM Authentication** — Login uses the server's actual Linux OS credentials (verified via `libpam` against `/etc/shadow`). No separate dashboard user management is needed.
- **JWT Session Tokens** — After a successful login, a signed JWT is issued. All API endpoints are heavily protected.
- **In-Memory Sessions** — Each server in the dashboard has its own login session, stored strictly in memory (`sessionStorage`). Sessions are cleared when the browser tab closes or when the user navigates back to the Server List.
- **Sudo Injection** — Commands requiring elevated privileges (`kill`, `systemctl`, `reboot`, `ufw`, `useradd`) securely use the user's authenticated password injected via `sudo -S` stdin — the password is never stored on disk or logged.

---

## Tech Stack

| Layer | Technology |
|---|---|
| **Backend** | Rust, Axum 0.8, Tokio, SQLx (SQLite), portable-pty |
| **Metrics Source** | `/proc/stat`, `/proc/meminfo`, `/proc/uptime`, `/proc/mounts`, `/proc/net/dev`, `statvfs()` |
| **Auth & Security** | PAM (`libpam`), JWT (`jsonwebtoken`) |
| **Frontend** | Vue 3, Vite, Tailwind CSS v4, Chart.js, xterm.js, Lucide Icons |
| **Deployment** | Backend: bare-metal binary · Frontend: Nginx via Docker/Podman Compose |

---

## OS Support

Designed specifically for **Linux** environments due to direct `/proc` and `/sys` filesystem access, as well as Linux-specific tools like systemctl, journalctl, and UFW.

| Distro | Status |
|---|---|
| Arch Linux / CachyOS / Manjaro | ✅ Fully Supported & Tested |
| Ubuntu / Debian | ✅ Fully Supported & Tested |
| Windows (Native) | ❌ Unsupported |
| macOS | ❌ Unsupported |

---

## Prerequisites

### Backend Host (bare-metal)

```bash
# Arch Linux / CachyOS / Manjaro
sudo pacman -S rustup pam zip unzip
rustup default stable
sudo pacman -S iproute2 podman nmap wget ufw speedtest-cli

# Ubuntu / Debian
sudo apt install curl build-essential libpam0g-dev zip unzip
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install iproute2 podman nmap wget ufw speedtest-cli
```

> **Note:** `libpam0g-dev` is required to compile the PAM authentication module. `zip` and `unzip` are required for File Explorer archive actions.

### Frontend Host

```bash
# Docker Compose or Podman Compose
docker --version   # or: podman --version
```

---

## Deployment Architecture

```
┌─────────────────────────────────────┐    ┌────────────────────────────────┐
│        Frontend (Vue.js SPA)        │    │     Backend Agent (Rust)       │
│  Hosted anywhere via Nginx/Podman   │◄──►│  Must run on bare-metal Linux  │
│  http://your-dashboard-host:3000    │    │  http://<SERVER_IP>:8080       │
└─────────────────────────────────────┘    └────────────────────────────────┘
                                                        │
                                                Reads from kernel:
                                                /proc/stat, /proc/meminfo
                                                /proc/net/dev, /etc/passwd
                                                Execute: systemctl, journalctl
```

The frontend is a standard Single Page Application (SPA) that can be hosted anywhere. You can add **multiple Backend Agent IPs** into a single Frontend dashboard to manage a fleet of servers.

---

## Backend Setup

### 1. Configure `.env`

```env
PORT=8080
FILE_ROOT=/home/user/Documents
DB_PATH=sqlite:./data.db
JWT_SECRET=your-random-super-strong-secret-string-here
```

### 2. Build & Run

```bash
chmod +x start.sh
./start.sh
```

This script automatically runs `cargo build --release` and spawns the backend daemon in the background. Logs are written to `server.log`. Database migrations are applied automatically.

---

## Frontend Setup

### Via Podman Compose (Recommended)

From the project root directory:

```bash
podman compose up -d --build
# or using Docker
docker compose up -d --build
```

Open `http://localhost:3000` in your browser.

### Via npm (Development)

```bash
cd frontend-vue
npm install
npm run dev
```

---

## Adding a Server

1. Open the dashboard and click **"Add Server"** (top right of Home page).
2. Fill in:
   - **Server Alias** — A friendly name (e.g. "VPS Singapore")
   - **Backend IP / URL** — Just enter the IP:Port (e.g. `100.127.55.109:8080`).
   - **Username** — Your Linux OS username on that server (must have sudo privileges for full features).
   - **Password** — Your Linux OS password.
3. Click **"Add & Login"** — The dashboard authenticates against the server's PAM system immediately. If successful, you are securely logged in and redirected to the server dashboard.

---

## License

MIT License
