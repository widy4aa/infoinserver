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

### User & Group Management
- **Linux User Management** — Create users (`useradd`), change passwords, assign to groups, delete users (with optional home dir removal)
- **Group Management** — Create and delete Linux groups, view all group members
- **SSH Key Manager** — Per-user management of `~/.ssh/authorized_keys`: list, add (with format validation for ssh-rsa/ed25519/ecdsa), and delete individual keys

### File Management
- **Full Filesystem Explorer** — Browse the entire Linux filesystem from `/`, not just a sandboxed home dir
- **3 View Modes** — List, Grid (with image thumbnails), and Compact mode — persisted per browser session
- **Recursive Search** — Find files across subdirectories via `find`, up to 8 levels deep
- **Disk & Removable Media Sidebar** — View all block devices (fixed + removable), usage bars, mount/unmount USB drives directly from the UI
- **File Actions** — Rename, Move, Copy, Delete, Compress to ZIP (with optional password), Extract ZIP, chmod (visual checkbox UI)
- **File Info Modal** — Size, permissions (octal + symbolic), owner (user:group), last modified time
- **Text Editor** — Built-in browser editor for reading and writing text files
- **URL Fetch** — Download files from the internet directly to the server via `wget`
- **Upload & Download** — Multi-file upload via drag-and-drop, direct download for any file
- **Permission Zones** — Write is allowed only in `$HOME` and mounted removable drives; all other paths are read-only

### Network & Security
- **Network Interfaces** — Active interfaces, MAC addresses, IP networks, and RX/TX traffic — including default gateway per interface
- **Listening Ports** — All locally listening ports and their processes via `/proc/net`, with force-kill action
- **Deep Port Scan** — On-demand async port scanning via `nmap` with a background job queue and polling
- **UFW Firewall** — View status, toggle enable/disable, manage allow/deny rules
- **Internet Speedtest** — On-demand network speed testing via `speedtest-cli` with persistent history
- **Real-time Latency Indicator** — Live ping from the browser to the backend server, displayed in the navbar

### Intrusion Prevention (Fail2Ban)
- **Status Dashboard** — Installed jails, banned IP count per jail, auto-detect active/inactive
- **Manual Ban & Unban** — Ban or unban any IP from any jail directly from the UI
- **Jail Configuration** — Visual editor for `/etc/fail2ban/jail.local` with:
  - Service template catalog (SSH, Nginx, Postfix, Apache, WordPress, etc.)
  - Filter dropdown populated from `/etc/fail2ban/filter.d/`
  - Field tooltips explaining maxretry, bantime, findtime
  - Live preview of the generated INI config block
  - Delete jail button
- **Live Activity Log** — Tail `/var/log/fail2ban.log` in real-time from the UI
- **One-click Install** — Install fail2ban via `apt` or `pacman` directly from the dashboard

### Container & Compose Management
- **Multi-runtime Support** — Automatically detects Docker or Podman on the host
- **Container Management** — List, start, stop, remove containers; inspect details; view logs (tail N)
- **Docker/Podman Compose** — Deploy new projects via YAML editor, view per-service status, stop/restart/rebuild/scale/delete projects, edit YAML inline
- **Deploy Wizard** — Form-based container creation (name, image, ports, env vars, volumes, restart policy)

### Cloudflare Tunnel Management
- **Command Center UI** — Two-panel layout: control panel (left) + tab-based content (right)
- **Full Setup Wizard** — Step-by-step wizard: Install → Authorize (capture URL) → Create Tunnel
- **Control Panel** — Tunnel name, running status with animated dot, version, UUID, uptime, quick stats (routes, active DNS, uptime), service start/stop/restart, danger-zone delete
- **Routes Tab** — Unified table (Ingress + CNAME in one row), inline actions, add route form
- **Health Tab** — Per-domain HTTP probe with detailed diagnostics: HEALTHY, ERR_502 (local service down), ERR_1033 (tunnel daemon down), NXDOMAIN (DNS missing)
- **Live Logs Tab** — WebSocket stream from `journalctl -f` for cloudflared, with filter (all/err/wrn), pause/resume, clear
- **CNAME DNS Manager** — Register DNS CNAME to Cloudflare, track status in local SQLite (instant green — no DNS propagation wait), delete CNAME from Cloudflare

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
- **Write Permission Zones** — The file manager enforces a zone model: write is only allowed inside `$HOME` and mounted removable drives. All other filesystem paths are read-only from the UI
- **Sudo Injection** — Commands requiring elevated privileges securely inject the authenticated password via `sudo -S` stdin. The password is never stored on disk, never logged, and never sent back to the frontend

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
| **Deployment** | Backend: bare-metal Rust binary · Frontend: built static files served by backend or Nginx/Podman |

---

## OS Support

InfoIn Server reads directly from Linux kernel interfaces. It is a Linux-first application.

| Distro | Status |
|---|---|
| **Arch Linux / CachyOS / Manjaro** | ✅ Fully Supported & Tested |
| **Ubuntu / Debian** | ✅ Fully Supported & Tested |
| **Fedora / Rocky / AlmaLinux** | ⚠️ Should work, not officially tested |
| **NixOS / Alpine** | ⚠️ Experimental |
| **Windows (native)** | ❌ Unsupported |
| **macOS** | ❌ Unsupported |

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

> **Ubuntu/Debian note:** `libpam0g-dev` is required to compile the PAM authentication module. `libclang-dev` is required by `pam-sys` (bindgen). TLS uses pure-Rust `ring` crypto — no `libssl-dev` needed.

### Frontend Host

The frontend is a standard Vue 3 SPA built with Vite. It can be served from the same machine or any static file host.

```bash
# Node.js (required for building)
node --version   # Required: v22.18.0 or >= v24.12.0

# Package manager
npm --version
```

For containerized frontend hosting:

```bash
docker --version   # or: podman --version
```

---

## Environment Variables

Create a `.env` file in the project root before running the backend:

```env
# Port the backend HTTP server listens on
PORT=8080

# Root directory for the file manager.
# $HOME is expanded at runtime to the home directory of the OS user running the backend.
# To restrict access to a subdirectory: FILE_ROOT=/home/username/projects
FILE_ROOT=$HOME

# Path to the SQLite database file
DB_PATH=sqlite:./data.db

# Secret key for signing JWT tokens. Change this to a strong random string in production.
# Generate one: openssl rand -hex 32
JWT_SECRET=change-this-to-a-strong-random-secret
```

> **Security note:** `JWT_SECRET` is not in the default `.env` file but is read from the environment at startup. If not set, it falls back to `"changeme-jwt-secret"` which is insecure for production use. Always set this variable in production.

---

## Database Schema

InfoIn Server uses a single SQLite database file (`data.db`) with 5 tables, created automatically at startup via `src/db/migrations.sql`.

| Table | Purpose |
|---|---|
| `system_metrics_history` | CPU, RAM, Disk, Network stats recorded every 5 minutes |
| `speedtest_history` | Results of internet speed tests (download/upload Mbps, ping) |
| `port_scan_jobs` | Async nmap scan jobs with status (pending/running/done/failed) and results |
| `activity_log` | Audit trail of all admin actions taken via the dashboard |
| `cloudflare_cname_status` | Local record of DNS CNAME registrations for Cloudflare Tunnel routes |

---

## Deployment

### Option A: Backend on bare-metal + Frontend in container

This is the recommended setup. The backend runs directly on the host to have full access to system commands and kernel interfaces. The frontend runs in a container for isolation.

```bash
# 1. Configure environment
cp .env.example .env
nano .env   # Set JWT_SECRET and other values

# 2. Build and start the backend
chmod +x start.sh
./start.sh   # Runs cargo build --release, spawns daemon, writes to server.log

# 3. Start the frontend container
podman compose up -d --build
# or:
docker compose up -d --build

# Frontend is now available at: http://localhost:3000
# Backend API is at:             http://localhost:8080
```

### Option B: Backend serves frontend directly (single binary)

Build the Vue frontend into the `static/` directory, and the Axum backend will serve it as a fallback:

```bash
# Build frontend
cd frontend-vue
npm install
npm run build   # Outputs to ../static/

# Run backend (serves both API and frontend)
cd ..
./start.sh

# Dashboard available at: http://localhost:8080
```

### Option C: Development mode

```bash
# Terminal 1: Start the backend with hot-reload
cargo watch -x run   # or: ./start.sh

# Terminal 2: Start the frontend dev server
cd frontend-vue
npm install
npm run dev

# Frontend dev server: http://localhost:5173 (proxies API to :8080)
```

---

## Adding a Server

1. Open the dashboard at `http://localhost:3000` (or your configured URL)
2. Click **Settings** and then **Add Server**
3. Fill in:
   - **Server Alias** — A friendly name (e.g., "VPS Singapore")
   - **Backend URL** — The full URL including port (e.g., `http://100.127.55.109:8080`)
   - **Username** — Your Linux OS username on that server (must have `sudo` privileges for full features)
   - **Password** — Your Linux OS password
4. Click **Add & Login** — The dashboard authenticates against the server's PAM system immediately

You can add **multiple backend servers** to a single frontend dashboard and switch between them.

---

## API Endpoints

InfoIn Server exposes a REST API with 80+ endpoints. All protected endpoints require a JWT token in the `Authorization: Bearer <token>` header. WebSocket endpoints accept `?token=<token>` as a query parameter.

### Public Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/ping` | Health check — returns `"pong"` |
| `POST` | `/api/auth/login` | Authenticate via PAM, returns JWT token |

### System Metrics

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/metrics/ws` | **WebSocket** — Live system metrics every 3 seconds |
| `GET` | `/api/metrics/history` | Historical metrics for the last 24 hours |

### System Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/system/os_updates` | Check for pending OS package updates |
| `GET` | `/api/system/os_updates/ws` | **WebSocket** — Live stream of OS upgrade process |
| `POST` | `/api/system/update` | Self-update the dashboard (git pull + rebuild) |
| `POST` | `/api/system/reboot` | Reboot the server |
| `GET` | `/api/syslogs` | System journal logs (`?filter=all\|auth\|kernel`) |
| `GET` | `/api/process/list` | Top processes by CPU/RAM |
| `POST` | `/api/process/kill/{pid}` | Kill a process by PID |
| `GET` | `/api/services` | List all systemd services |
| `POST` | `/api/services/action` | Start, stop, restart, enable, or disable a service |
| `GET` | `/api/cron` | Read root's crontab |
| `POST` | `/api/cron` | Update root's crontab |

### Users & Groups

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/users` | List all Linux users |
| `POST` | `/api/users` | Create a new Linux user |
| `PUT` | `/api/users/{username}/password` | Change user password |
| `PUT` | `/api/users/{username}/groups` | Update user's secondary groups |
| `DELETE` | `/api/users/{username}` | Delete a user (`?remove_home=true` to also remove home dir) |
| `GET` | `/api/users/{username}/ssh` | List SSH authorized keys for a user |
| `POST` | `/api/users/{username}/ssh` | Add SSH public key for a user |
| `DELETE` | `/api/users/{username}/ssh` | Remove an SSH key |
| `GET` | `/api/groups` | List all Linux groups |
| `POST` | `/api/groups` | Create a new group |
| `DELETE` | `/api/groups/{groupname}` | Delete a group |

### File System

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/files/config` | Get file manager config (home root, system root) |
| `GET` | `/api/files/list` | List directory contents (`?path=`) |
| `GET` | `/api/files/search` | Search files recursively (`?path=&query=`) |
| `GET` | `/api/files/download` | Download a file (`?path=`) |
| `POST` | `/api/files/upload` | Upload files to a directory (`?path=`, multipart) |
| `POST` | `/api/files/fetch` | Download a URL to the server via wget |
| `POST` | `/api/files/action` | File operations: rename, move, copy, delete, compress, extract, chmod |
| `POST` | `/api/files/text` | Read or write a text file |
| `GET` | `/api/files/info` | Get file metadata: size, permissions, owner, modified time |
| `GET` | `/api/disk/info` | List all block devices with usage stats |
| `POST` | `/api/disk/mount` | Mount a device (`{ device, label }`) |
| `POST` | `/api/disk/umount` | Unmount a device (`{ device }`) |

### Network & Security

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/network` | Network interfaces, IPs, gateway, RX/TX stats |
| `GET` | `/api/ports` | All locally listening ports |
| `POST` | `/api/ports/scan` | Trigger async nmap scan (`{ target }`) |
| `GET` | `/api/ports/scan/{job_id}` | Get nmap scan status and results |
| `GET` | `/api/firewall/status` | UFW firewall status and rules |
| `POST` | `/api/firewall/toggle` | Enable or disable UFW |
| `POST` | `/api/firewall/rule` | Add or remove a UFW rule (`{ action, port }`) |
| `GET` | `/api/speedtest/history` | Last 20 speedtest results |
| `POST` | `/api/speedtest/run` | Run a new speedtest |

### Fail2Ban

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/fail2ban/status` | Jail status and banned IPs |
| `POST` | `/api/fail2ban/install` | Install fail2ban via apt or pacman |
| `POST` | `/api/fail2ban/ban` | Manually ban an IP in a jail |
| `POST` | `/api/fail2ban/unban` | Unban an IP from a jail |
| `GET` | `/api/fail2ban/logs` | Last 100 lines of `/var/log/fail2ban.log` |
| `GET` | `/api/fail2ban/config` | Read and parse `/etc/fail2ban/jail.local` |
| `POST` | `/api/fail2ban/config` | Save or update a jail configuration |
| `DELETE` | `/api/fail2ban/config/{name}` | Delete a jail from the config |
| `GET` | `/api/fail2ban/filters` | List available filters from `filter.d/` |

### Cloudflare Tunnel

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/cloudflare/status` | Full tunnel status (install, service, auth, tunnel name, uptime) |
| `POST` | `/api/cloudflare/install` | Install cloudflared binary |
| `POST` | `/api/cloudflare/login` | Start `cloudflared tunnel login`, capture auth URL |
| `GET` | `/api/cloudflare/login/status` | Poll for successful authorization (cert.pem check) |
| `POST` | `/api/cloudflare/create` | Create a named tunnel |
| `DELETE` | `/api/cloudflare/tunnel` | Permanently delete the tunnel |
| `POST` | `/api/cloudflare/start` | Start the cloudflared service |
| `POST` | `/api/cloudflare/stop` | Stop the cloudflared service |
| `POST` | `/api/cloudflare/restart` | Restart the cloudflared service |
| `GET` | `/api/cloudflare/config` | Read and parse `config.yml` (with CNAME status from DB) |
| `POST` | `/api/cloudflare/routes` | Add an ingress route to `config.yml` |
| `DELETE` | `/api/cloudflare/routes` | Delete an ingress route from `config.yml` |
| `POST` | `/api/cloudflare/routes/dns` | Register a DNS CNAME for a hostname |
| `GET` | `/api/cloudflare/health` | HTTP probe all configured hostnames |
| `GET` | `/api/cloudflare/logs` | Last 100 lines from cloudflared journalctl |
| `GET` | `/api/cloudflare/logs/ws` | **WebSocket** — Live cloudflared log stream |

### Containers & Compose

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/container/runtime` | Detected container runtime (Docker or Podman) |
| `GET` | `/api/container/list` | List all containers |
| `POST` | `/api/container/create` | Create and start a container |
| `POST` | `/api/container/{action}/{id}` | Container action: start, stop, rm, restart |
| `GET` | `/api/container/inspect/{id}` | Inspect container details |
| `GET` | `/api/container/logs/{id}` | Container logs (`?tail=N`) |
| `GET` | `/api/compose/projects` | List all compose projects |
| `POST` | `/api/compose/deploy` | Deploy a new compose project |
| `POST` | `/api/compose/{name}/stop` | Stop a project |
| `POST` | `/api/compose/{name}/restart` | Restart a project |
| `POST` | `/api/compose/{name}/rebuild` | Rebuild and redeploy a project |
| `GET` | `/api/compose/{name}/ps` | Per-service status of a project |
| `GET` | `/api/compose/{name}/logs` | Project logs (`?service=&tail=N`) |
| `POST` | `/api/compose/{name}/scale` | Scale a service to N replicas |
| `GET` | `/api/compose/{name}/yaml` | Get the compose YAML |
| `PUT` | `/api/compose/{name}/yaml` | Update compose YAML and redeploy |
| `DELETE` | `/api/compose/{name}` | Delete a compose project |

### Logs & Terminal

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/logs/activity` | Last 200 dashboard activity log entries |
| `GET` | `/api/logs/bash_history` | Bash command history for root and current user |
| `GET` | `/api/terminal/ws` | **WebSocket** — Interactive PTY terminal (`/bin/bash`) |

---

## License

MIT License — see `LICENSE` for details.

---

*InfoIn Server is an independent open-source project. It is not affiliated with, endorsed by, or derived from any commercial monitoring product.*
