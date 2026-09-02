<p align="center">
  <img src="frontend-vue/public/server-icon-blue.svg" width="96" alt="InfoIn Server" />
</p>

# InfoIn Server

> **Your Linux server. Fully visible. Fully in control.**

A self-hosted, open-source server monitoring and administration dashboard built specifically for Linux bare-metal machines. InfoIn Server reads metrics directly from the kernel — `/proc`, `/sys`, `/dev` — streams them live over WebSocket, authenticates via your OS's own PAM system, and gives you a modern web interface to manage every aspect of your server without SSH gymnastics.

No agents. No telemetry. No cloud dependency. Just your server, talking to you.

---

## Philosophy

InfoIn Server was built around three foundational ideas drawn from computer science literature:

### 1. The Unix Philosophy — *Do one thing, and do it well*
> *"Write programs that do one thing and do it well. Write programs to work together."*
> — Doug McIlroy, as quoted in Eric S. Raymond's *The Art of Unix Programming* (2003)

Each component in InfoIn Server is a focused module. The backend does not try to be an ORM, a scheduler framework, or a plugin platform. It reads the kernel, executes trusted system commands, and streams data — nothing more. The frontend renders that data beautifully.

### 2. Cognitive Load Theory — *Complexity kills usability*
> *"Working memory has a limited capacity... instruction should be designed to reduce unnecessary cognitive load."*
> — John Sweller, *Cognitive Load During Problem Solving: Effects on Learning*, Cognitive Science (1988)

Every UI decision in InfoIn Server minimizes the mental overhead of server administration. Status is always visible. Actions are always contextual. You should never need to read a manual to restart a service or ban an IP.

### 3. The Pragmatic Programmer — *Own your tools*
> *"Use a single editor well... The editor should be an extension of your hand."*
> — Andrew Hunt & David Thomas, *The Pragmatic Programmer: 20th Anniversary Edition* (2019)

A monitoring dashboard is a tool. If you cannot extend it, trust it, or understand it, it owns you. InfoIn Server is fully self-hosted, fully readable, and fully yours. The source code is the documentation.

---

## Features

### Real-time Monitoring
- **Live System Metrics via WebSocket** — CPU, RAM, Disk, and Network streamed from `/proc` every 3 seconds with automatic reconnection
- **Historical Performance Charts** — CPU, RAM, Disk, and Network (Mbps) recorded every 5 minutes to SQLite, displayed as interactive line charts with time range filters (1h, 3h, 6h, 12h, 24h)
- **System Info Panel** — Hostname, OS name with distro icon, kernel version, uptime, logged-in user

### System Administration
- **Systemd Services** — Full UI to list, start, stop, restart, enable, and disable background daemons
- **Process Manager** — Live top-process list (CPU/RAM sorted), search by name or PID, force-kill any process
- **OS Package Updates** — Detect upgradable packages via `apt` or `checkupdates` (pacman), apply upgrades with a live streaming terminal via WebSocket
- **Cron Job Manager** — Read, add, edit, and delete `/etc/crontab` jobs via a structured form UI
- **System Reboot & Update** — One-click server reboot and dashboard self-update via `git pull + cargo build`
- **System Reset** — One-click reset for Cloudflare tunnel (full config + cert.pem), UFW firewall (all rules), and Fail2Ban (jail.local) from the Settings page

### User & Group Management
- **Multi-user Dashboard Session** — Switch between different sudo users securely without logging out. Sessions are stored and handled gracefully. Only non-root users with `sudo` or `wheel` privileges are allowed
- **Linux User Management** — Create users (`useradd`), change passwords securely, assign to groups, delete users (with optional home dir removal)
- **Group Management** — Create and delete Linux groups, view all group members
- **SSH Key Manager** — Per-user management of `~/.ssh/authorized_keys`: list, add (with format validation for ssh-rsa/ed25519/ecdsa), and delete individual keys

### Advanced File Management (File Explorer 2.0)
- **Full Filesystem Access** — Browse the entire Linux filesystem from `/`. Write operations are strictly restricted to your `$HOME` folder and connected USB drives. System folders (like `/etc`, `/var`) are read-only
- **Disk & Removable Media Sidebar** — Auto-detect internal disks and USB/SD Cards. View usage bars, browse partitions, and Mount/Eject removable media via the UI
- **3 View Modes** — List, Grid (with image thumbnails), and Compact mode — persisted per browser session
- **Recursive Search** — Find files across subdirectories via `find` command, up to 8 levels deep
- **File Actions** — Rename, Move, Copy (even across disks), Delete, Compress to ZIP (with optional password), Extract ZIP, visual Checkbox UI for `chmod`
- **File Info Modal** — Size, permissions (octal + symbolic), owner (user:group), last modified time
- **Text Editor** — Built-in browser editor for reading and writing text files
- **Upload & URL Fetch** — Multi-file upload via drag-and-drop, or download files from the internet directly to the server via `wget`

### Network & Security
- **Network Interfaces** — Active interfaces, MAC addresses, IP networks, default gateway, and visual RX/TX usage bars
- **Listening Ports** — Clean list of all locally listening ports with Scope Badges (🌍 Public / 🔒 Local), protocol, clean process name, and force-kill action
- **Deep Port Scan** — On-demand async port scanning via `nmap` with structured result tables, highlighting dangerous open ports
- **UFW Firewall** — View status, toggle enable/disable with one click, manage allow/deny rules
- **Internet Speedtest** — On-demand network speed testing via `speedtest-cli` with a 5-history FIFO retention and large summary UI
- **Real-time Latency Indicator** — Live ping (e.g., `🟢 12ms`) from the browser to the backend server, displayed directly in the navbar

### Intrusion Prevention (Fail2Ban Command Center)
- **Status Dashboard** — Installed jails, banned IP count per jail, auto-detect active/inactive
- **Manual Ban & Unban** — Ban or unban any IP from any jail using a clean inline form
- **Jail Configuration** — Visual editor for `/etc/fail2ban/jail.local` with:
  - **Quick Service Templates**: SSH, Nginx, Postfix, Apache, WordPress, etc.
  - Filter dropdown automatically populated from `/etc/fail2ban/filter.d/`
  - Field tooltips explaining maxretry, bantime, findtime
  - Live preview of the generated INI config block
  - Delete jail button
- **Live Activity Log** — Tail `/var/log/fail2ban.log` in real-time to watch bans happen live
- **One-click Install** — Install fail2ban via `apt` or `pacman` directly from the dashboard

### Container & Compose Management
- **Multi-runtime Support** — Automatically detects Docker or Podman on the host
- **Container Management** — List, start, stop, remove containers; inspect details; view logs
- **Docker/Podman Compose** — Deploy new projects via YAML editor, view per-service status, stop/restart/rebuild/scale/delete projects, edit YAML inline

### Cloudflare Tunnel Management
- **Command Center UI** — Two-panel layout: Control panel (left) + Tab-based content (right)
- **Full Setup Wizard** — Step-by-step wizard: Install → Authorize (capture URL) → Create Tunnel
- **Control Panel** — Tunnel name, animated running status, version, UUID, quick stats (routes, active DNS, uptime), service start/stop/restart, and danger-zone delete
- **Routes Tab** — Unified table for Ingress + CNAME status, inline actions, add route form
- **Health Tab** — Per-domain HTTP probe with accurate E2E diagnostics: HEALTHY, ERR_502 (local service down), ERR_1033 (tunnel daemon down), NXDOMAIN (DNS truly missing — distinguished from connection errors)
- **Live Logs Tab** — WebSocket stream from `journalctl -f` for cloudflared, with filter (all/err/wrn), pause/resume, clear
- **CNAME DNS Manager** — Register DNS CNAME to Cloudflare, tracked via local SQLite for instant status feedback

### UI & Accessibility
- **Dark Mode** — Full dark mode support with Moon/Sun toggle in the navbar. Preference persisted to `localStorage`
- **Safety Status Colors** — Critical pages (dashboard, services, containers, Cloudflare) use semantic color indicators: emerald (healthy/online), yellow (degraded/pending), red (failed/down) with glow effects for instant visual scanning
- **Responsive Layout** — Works on any screen size; tables auto-truncate long names without horizontal scroll

---

### System Logs & Audit
- **System Journal** — `journalctl` viewer with filter modes: All, Auth (SSH), Kernel (dmesg). Live auto-refresh with pause/resume
- **Dashboard Audit Log** — Every action taken through the dashboard (firewall changes, user creation, tunnel deletion, etc.) is recorded to SQLite with level (INFO / WARNING / CRITICAL), action name, and detail
- **Bash History Viewer** — Read `/root/.bash_history` and user bash histories in a terminal-style viewer

### Developer Tools
- **Multi-session Root Terminal** — Native PTY shell via WebSocket (`xterm.js`). Multiple independent browser tabs each get their own `/bin/bash` shell
- **OS Update Terminal** — Live streaming terminal for `apt upgrade` or `pacman -Syu` via WebSocket

### Security Architecture
- **PAM Authentication** — Login uses the real Linux OS credentials, verified via `libpam` against `/etc/shadow`. No separate user database needed
- **JWT Session Tokens** — After successful login, a signed JWT is issued. All API endpoints are protected. WebSocket connections use `?token=` query parameter
- **Auto-Logout on Auth Failure** — If a `sudo` command fails due to an expired or invalid session, the API returns a recognizable error string, the frontend detects it, clears the token, and redirects to the login screen automatically
- **Write Permission Zones** — The file manager enforces a strict zone model: write is only allowed inside your `$HOME` and mounted removable drives. All other filesystem paths are read-only
- **Sudo Injection** — Commands requiring elevated privileges securely inject the authenticated password via `sudo -S` stdin without pipe buffering issues. The password is never stored on disk, never logged, and never sent back to the frontend

---

## Tech Stack

| Layer | Technology |
|---|---|
| **Backend** | Rust, Axum 0.8, Tokio (async runtime), SQLx + SQLite, portable-pty |
| **Metrics Source** | `/proc/stat`, `/proc/meminfo`, `/proc/uptime`, `/proc/mounts`, `/proc/net/dev`, `statvfs()`, `sysinfo` crate |
| **Auth & Security** | PAM (`libpam`), JWT (`jsonwebtoken`), bcrypt-compatible |
| **Frontend** | Vue 3 (Composition API), Vite 8, Tailwind CSS v4, Chart.js, xterm.js, Lucide Icons |
| **WebSocket** | Axum WS + Tokio for 4 real-time channels: metrics, terminal PTY, Cloudflare logs, OS upgrade |
| **Database** | SQLite (via SQLx async) — 5 tables: metrics history, speedtest history, port scan jobs, activity log, CNAME status |

---

## OS Support

InfoIn Server reads directly from Linux kernel interfaces. It is a Linux-first application.

| Distro | Status |
|---|---|
| **Arch Linux / CachyOS / Manjaro** | ✅ Fully Supported & Tested |
| **Ubuntu / Debian** | ✅ Fully Supported & Tested |
| **Fedora / Rocky / AlmaLinux** | ⚠️ Should work, not officially tested |
| **NixOS / Alpine** | ⚠️ Experimental |
| **Windows (native) / macOS** | ❌ Unsupported |

---

## Prerequisites

### Backend Host (bare-metal Linux)

#### Arch Linux / CachyOS / Manjaro

```bash
# Build tools & core
sudo pacman -S rustup base-devel clang pam zip unzip

# Set up Rust toolchain
rustup default stable

# Runtime tools used by the dashboard
sudo pacman -S iproute2 nmap wget ufw speedtest-cli fail2ban
sudo pacman -S podman docker-compose   # For container management (optional)
```

> **Pacman note:** `fail2ban` may require `pacman-contrib` for the `checkupdates` command used in OS update checks.

#### Ubuntu / Debian

```bash
# Build dependencies
sudo apt update
sudo apt install -y \
  curl build-essential pkg-config \
  libclang-dev libpam0g-dev \
  zip unzip

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Runtime tools used by the dashboard
sudo apt install -y \
  iproute2 nmap wget ufw speedtest-cli \
  fail2ban

# For OS package update feature (Debian/Ubuntu)
# apt-get is already available — no extra install needed

# For container management (optional)
sudo apt install -y podman docker-compose
```

### Frontend Host

The frontend is a standard Vue 3 SPA built with Vite. It can be served from the same machine or any static file host.

```bash
# Node.js (required for building)
node --version   # Required: v22.18.0 or >= v24.12.0

# Package manager
npm --version
```

---

## Environment Variables

Create a `.env` file in the project root before running the backend:

```env
# Port the backend HTTP server listens on
PORT=8080

# Root directory for the file manager.
# $HOME is dynamically expanded to the home directory of the ACTIVE user session in the dashboard.
FILE_ROOT=$HOME

# Path to the SQLite database file
DB_PATH=sqlite:./data.db

# Secret key for signing JWT tokens. Change this to a strong random string in production.
# Generate one: openssl rand -hex 32
JWT_SECRET=change-this-to-a-strong-random-secret
```

---

## Database Schema

InfoIn Server uses a single SQLite database file (`data.db`) with 5 tables, created automatically at startup.

| Table | Purpose |
|---|---|
| `system_metrics_history` | CPU, RAM, Disk, Network stats recorded every 5 minutes |
| `speedtest_history` | Results of internet speed tests (download/upload Mbps, ping). Keeps a maximum of 5 recent tests (FIFO) |
| `port_scan_jobs` | Async nmap scan jobs with status and results |
| `activity_log` | Audit trail of all admin actions taken via the dashboard |
| `cloudflare_cname_status` | Local record of DNS CNAME registrations for Cloudflare Tunnel routes |

---

## Deployment

### Option A: Backend on bare-metal + Frontend in container (Recommended)

```bash
# 1. Configure environment
cp .env.example .env
nano .env   # Set JWT_SECRET and other values

# 2. Build and start the backend
chmod +x start.sh
./start.sh   # Runs cargo build --release, spawns daemon, writes to server.log

# 3. Start the frontend container
podman compose up -d --build
# or: docker compose up -d --build

# Frontend is now available at: http://localhost:3000
# Backend API is at:             http://localhost:8080
```

### Option B: Backend serves frontend directly (single binary)

Build the Vue frontend into the `static/` directory, and the Axum backend will serve it as a fallback:

```bash
cd frontend-vue
npm install
npm run build   # Outputs to ../static/
cd ..
./start.sh

# Dashboard available at: http://localhost:8080
```

---

## Adding a Server

1. Open the dashboard at `http://localhost:3000`
2. Click **Settings** and then **Add Server**
3. Fill in the Server Alias and Backend URL (e.g., `http://100.127.55.109:8080`).
4. Click **Add & Login**. You must log in using an OS user account that belongs to the `sudo` or `wheel` group. Root logins are explicitly prohibited for security.

You can add **multiple backend servers** to a single frontend dashboard and switch between them. Furthermore, the dashboard supports adding **multiple OS users** to a single server connection, allowing you to quickly hot-swap between users (e.g. from `infratek` to `webmaster`) using the profile dropdown.

---

## API Endpoints

InfoIn Server exposes a REST API with over 85 endpoints. All protected endpoints require a JWT token in the `Authorization: Bearer <token>` header.

*(Key areas: `auth`, `metrics`, `system_mgmt`, `users_mgmt`, `services_mgmt`, `syslogs`, `cron_mgmt`, `ports`, `disk`, `speedtest`, `firewall`, `fail2ban`, `cloudflare`, `container`, `compose`, `files`)*

**New endpoints added:**

| Endpoint | Method | Description |
|---|---|---|
| `/api/firewall/reset` | POST | Reset UFW to default: disable + remove all rules |
| `/api/fail2ban/reset` | POST | Reset Fail2Ban: delete `jail.local` + restart service |
| `/api/cloudflare/reset` | POST | Full Cloudflare reset: tunnel, config, credentials, `cert.pem` |

---

## License

MIT License — see `LICENSE` for details.

---

*InfoIn Server is an independent open-source project. It is not affiliated with, endorsed by, or derived from any commercial monitoring product.*