# Server Monitoring Dashboard

A lightweight, single-binary server monitoring dashboard written in **Rust** (using Axum and Tokio). Designed to monitor system resources, network interfaces, listening ports, Podman containers, and run scheduled network speedtests—all accessible via a clean, web-based UI using HTTP polling.

## Features

- **System Metrics**: Real-time CPU usage, RAM utilization, Uptime, OS, and Kernel information.
- **Network Interface Info**: View active network interfaces, IP addresses, and bandwidth RX/TX traffic.
- **Listening Ports**: Instantly view locally listening ports and their associated processes via `ss`.
- **Deep Port Scan**: On-demand port scanning using `nmap` powered by an asynchronous background task queue.
- **Podman Integration**: List existing containers, check statuses, Start/Stop/Restart existing containers, and spin up new containers securely from the dashboard.
- **File Explorer**: A sandboxed web file browser to view and download files from a specific allowed directory (`FILE_ROOT`). Protected against Path Traversal vulnerabilities.
- **Speedtest**: Scheduled (hourly) and manual network speed testing using `speedtest-cli`, with results cached in a local SQLite database.
- **Terminal Access**: Integrated quick link to open a web terminal (assumes `ttyd` is running on port 7681).

## Tech Stack

- **Backend**: Rust, Axum, Tokio, Sqlx (SQLite), Sysinfo.
- **Frontend**: Vanilla HTML/JS/CSS (No heavy frameworks, No WebSockets required).

## OS Support

This dashboard is designed specifically for **Linux** environments due to its tight integration with Linux-native tools and the `/proc` filesystem. 
- **Fully Supported & Tested**: Arch Linux, CachyOS, Manjaro, Ubuntu, Debian.
- **Unsupported**: Windows (Native), macOS (Some features like `ss` or UFW will not work).

## Prerequisites

Before running the application, ensure your host machine has the following packages installed (depending on the features you want to use):

```bash
# Arch Linux / CachyOS / Manjaro
sudo pacman -S rustup
rustup default stable
sudo pacman -S iproute2 podman nmap wget ufw

# Ubuntu / Debian
sudo apt install curl build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install iproute2 podman nmap wget ufw
```

**Optional but Recommended (Auto-Installer supported by Dashboard):**
- `cloudflared` (for Secure Tunnels)
- `speedtest-cli` (for Network Benchmarks)

## Deployment Architecture

This project is built using a **Decoupled Architecture** allowing you to host the Backend (Server Monitor Agent) and the Frontend (Web Dashboard UI) completely separately.

### 1. Backend Agent (Rust)
The backend must run directly on the physical OS (bare-metal) to be able to scan native `/proc` directories, run `ss`, and interact natively with `podman` and `ufw`. 
It cannot be containerized via Docker effectively without mounting massive amounts of host privileges.

**Running the Backend on Bare-Metal:**
1. Configure your `.env`:
   ```env
   PORT=8080
   FILE_ROOT=/home/user/Documents
   AUTH_USER=admin
   AUTH_PASS=admin123
   DB_PATH=sqlite:./data.db
   ```
2. Build and start the binary:
   ```bash
   chmod +x start.sh
   ./start.sh
   ```
   *(This script runs `cargo build --release` and spawns the backend daemon in the background)*

### 2. Frontend UI (Vue.js)
The Frontend is a standard Single Page Application (Vue.js + Tailwind). It does not need any system privileges and can be hosted anywhere—even on a different continent! You can add multiple Backend Agent IPs into a single Frontend UI.

**Running the Frontend via Docker/Podman Compose:**
If you prefer keeping your web services containerized, you can use the provided `docker-compose.yml` to spin up the UI via Nginx.

1. Ensure Docker or Podman Compose is installed.
2. Run the deployment:
   ```bash
   # Using Docker
   docker-compose up -d

   # Using Podman
   podman-compose up -d
   ```
3. Open your browser and go to `http://localhost:3000`.
4. Navigate to the **Settings** tab in the UI to add your Backend Agent's IP address (e.g. `http://<SERVER_IP>:8080`).

---

- This dashboard executes shell commands (`podman`, `nmap`, `speedtest-cli`). While strict input validation and secure argument passing (`Command::new().args()`) are implemented to prevent command injection, **it is highly recommended to NOT expose this dashboard directly to the public internet.**
- Always run this application behind a Reverse Proxy (like Nginx or Caddy) configured with SSL/TLS and proper HTTP Basic Authentication or an external Auth provider (like Authelia).

## License

MIT License
