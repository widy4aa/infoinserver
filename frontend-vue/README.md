# frontend-vue

Vue 3 + Vite frontend for the Server Monitoring Dashboard.

## Project Structure

```
src/
├── assets/
│   └── main.css          # Global CSS — design system (buttons, cards, inputs)
├── components/
│   ├── LoginModal.vue     # PAM login modal (per-server auth)
│   ├── NativeTerminal.vue # Multi-tab PTY terminal (xterm.js + WebSocket)
│   └── ToastAlert.vue     # Toast notifications & confirm dialogs
├── composables/
│   └── useApi.js          # apiFetch() wrapper — auto-injects JWT Authorization header
├── router/
│   └── index.js           # Vue Router — routes for all views
├── stores/
│   ├── serverStore.js     # Server list, active server, token management (singleton state)
│   └── toastStore.js      # Toast/confirm dialog state
└── views/
    ├── HomeView.vue        # Server list homepage
    ├── ServerLayout.vue    # Per-server layout with tab navigation + auth guard
    ├── DashboardView.vue   # System metrics (WebSocket) + historical charts (Chart.js)
    ├── PortsView.vue       # Network interfaces + listening ports + Nmap scan
    ├── PodmanView.vue      # Container management
    ├── FilesView.vue       # File explorer
    ├── CloudflareView.vue  # Cloudflare Zero Trust tunnel management
    ├── SpeedtestView.vue   # Network speedtest history & manual run
    ├── LogsView.vue        # Activity logs & alerts
    └── SettingsView.vue    # Add server / server preferences / danger zone
```

## Design System

Button variants (defined in `main.css`):

| Class | Use |
|---|---|
| `btn-primary` | Main actions (blue) |
| `btn-secondary` / `btn-outline` | Secondary actions (slate) |
| `btn-danger` / `btn-destructive` | Destructive actions (red) |
| `btn-warning` | Caution actions (amber) |
| `btn-success` | Confirmatory actions (green) |
| `btn-icon-*` | Icon-only buttons in tables (green/amber/blue/red/slate) |

## Project Setup

```sh
npm install
```

### Development

```sh
npm run dev
```

### Build for Production

```sh
npm run build
```

### Build via Podman/Docker (Recommended)

From the project root:

```sh
podman compose up -d --build
# or
docker compose up -d --build
```

This builds the Vue app and serves it via Nginx on port 3000.

## Authentication Flow

1. User adds a server via **Settings → Add & Login** (fills IP, username, password).
2. Frontend calls `POST /api/auth/login` on the backend — credentials are verified via Linux PAM.
3. On success, a JWT token is returned and stored in `sessionStorage` (keyed by server ID).
4. All subsequent API calls via `apiFetch()` automatically include `Authorization: Bearer <token>`.
5. Token is cleared when the user clicks **Back** to the Home page (intentional logout).
6. Token survives browser refresh (F5) since it's in `sessionStorage`.
7. Token is gone when browser/tab is closed.

## IDE Setup

[VS Code](https://code.visualstudio.com/) + [Vue (Official)](https://marketplace.visualstudio.com/items?itemName=Vue.volar)
