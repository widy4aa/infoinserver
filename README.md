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

## Prerequisites

Before running the application, ensure your host machine has the following installed (depending on the features you want to use):
- `rust` and `cargo` (for building the app)
- `ss` (usually part of `iproute2` on Linux, used for fast port monitoring)
- `podman` (for container management features)
- `nmap` (for Deep Scan features)
- `speedtest-cli` (for network speed tests)

## Installation & Setup

1. **Clone the repository**
   ```bash
   git clone <repository_url>
   cd infoinserver
   ```

2. **Configure Environment Variables**
   Ensure the `.env` file exists in the root directory and contains the following configurations:
   ```env
   PORT=8080
   FILE_ROOT=/home/user/Documents # The directory the file explorer is restricted to
   AUTH_USER=admin                # For future basic auth implementation
   AUTH_PASS=admin123             # For future basic auth implementation
   DB_PATH=sqlite:./data.db       # Path to the SQLite database
   ```

3. **Run the Application**
   ```bash
   cargo run --release
   ```
   The backend will automatically create the `data.db` SQLite file and run the necessary schema migrations upon startup.

4. **Access the Dashboard**
   Open your browser and navigate to: `http://localhost:8080` (or the IP of your server).

## Security Notice

- This dashboard executes shell commands (`podman`, `nmap`, `speedtest-cli`). While strict input validation and secure argument passing (`Command::new().args()`) are implemented to prevent command injection, **it is highly recommended to NOT expose this dashboard directly to the public internet.**
- Always run this application behind a Reverse Proxy (like Nginx or Caddy) configured with SSL/TLS and proper HTTP Basic Authentication or an external Auth provider (like Authelia).

## License

MIT License
