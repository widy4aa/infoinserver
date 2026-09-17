# frontend-vue

Vue 3 + Vite frontend for the Server Monitoring Dashboard.

## Project Structure

```
src/
├── assets/
│   └── main.css          # Global CSS — design system (buttons, cards, inputs)
├── components/
│   ├── LoginModal.vue     # PAM login modal (per-server auth)
│   ├── LoginPanel.vue     # Login card with typewriter terminal effect
│   ├── FeatureHighlight.vue # Right-side feature list on the login page
│   ├── AddUserForm.vue    # Standalone add-user form component
│   ├── CloudflarePanel.vue# Cloudflare sub-panel component
│   ├── NativeTerminal.vue # Multi-tab PTY terminal (xterm.js + WebSocket)
│   └── ToastAlert.vue     # Toast notifications & confirm dialogs
├── composables/
│   ├── useApi.js          # apiFetch() wrapper — auto-injects JWT Authorization header
│   └── useHeartbeat.js    # Sends heartbeat every 30s to update presence last_seen
├── router/
│   └── index.js           # Vue Router — routes for all views
├── stores/
│   ├── serverStore.js     # Server list, active server, token management (singleton state)
│   ├── toastStore.js      # Toast/confirm dialog state
│   ├── themeStore.js      # Dark mode toggle, isDark ref, localStorage persistence
│   └── authStore.js       # GitHub session, role, canWrite, restrictedLabels, allowedLabels
└── views/
    ├── LoginView.vue       # Login page (layered background, LoginPanel, FeatureHighlight)
    ├── AuthCallbackView.vue# Handles GitHub OAuth redirect, stores session token
    ├── HomeView.vue        # Server list homepage
    ├── ServerLayout.vue    # Per-server layout with tab navigation + auth guard
    ├── DashboardView.vue   # System metrics (WebSocket) + historical charts (Chart.js)
    ├── PortsView.vue       # Network interfaces + listening ports + Nmap scan
    ├── ContainerView.vue   # Container management (Docker/Podman + Compose + VMs tab)
    ├── FilesView.vue       # File explorer
    ├── CloudflareView.vue  # Cloudflare Zero Trust tunnel management
    ├── CronView.vue        # Cron job manager (/etc/crontab structured form UI)
    ├── ServicesView.vue    # Systemd services + top processes
    ├── UsersView.vue       # Linux users, groups, SSH key management
    ├── SyslogsView.vue     # journalctl viewer + dashboard activity log + bash history
    ├── UpdatesView.vue     # OS package upgrades with live WebSocket terminal
    └── SettingsView.vue    # Server connection settings / system reset (also at /settings global route)
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

This builds the Vue app and serves it via **Bun + Hono** on port 3000.

## Authentication Flow

1. User navigates to the app and is redirected to `/login` (the dedicated login page).
2. User clicks **Sign in with GitHub** — browser is redirected to GitHub OAuth, handled by the Bun server.
3. After GitHub authorizes, Bun exchanges the code for an access token, fetches the GitHub profile, and signs a session JWT.
4. Browser is redirected to `/auth/callback` where `AuthCallbackView.vue` stores the session token in `localStorage`.
5. `authStore.js` fetches the user's role from `/api/roles/me` — role controls read/write access to the shared server config.
6. User adds a server via **Settings → Add Server** (fills in backend URL, username, password).
7. Frontend calls `POST /api/auth/login` on the Rust backend — credentials are verified via Linux PAM.
8. On success, a PAM JWT is returned and stored in `sessionStorage` (keyed by server ID).
9. All subsequent API calls via `apiFetch()` automatically include `Authorization: Bearer <pam_token>`.
10. PAM token is gone when the browser/tab is closed. GitHub session token persists until expiry (7 days).

## IDE Setup

[VS Code](https://code.visualstudio.com/) + [Vue (Official)](https://marketplace.visualstudio.com/items?itemName=Vue.volar)
