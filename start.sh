#!/bin/bash

# start.sh - Script untuk menjalankan InfoIn Server Dashboard
# Usage:
#   ./start.sh                     → start backend + frontend (production)
#   ./start.sh --backend           → start backend (Rust) saja
#   ./start.sh --frontend          → start frontend (Bun+Hono) saja, production
#   ./start.sh --frontend --dev    → start frontend dev mode (Vite HMR + Bun proxy)
#   ./start.sh --dev               → start backend + frontend dev mode

APP_NAME="infoinserver"
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="$APP_DIR/target/release/$APP_NAME"
FRONTEND_DIR="$APP_DIR/frontend-vue"

# ── Parse flags ───────────────────────────────────────────────────────────────
MODE=""       # backend | frontend | "" (keduanya)
DEV=false

for arg in "$@"; do
    case "$arg" in
        --backend)  MODE="backend" ;;
        --frontend) MODE="frontend" ;;
        --dev)      DEV=true ;;
        *)
            echo "Usage: $0 [--backend | --frontend] [--dev]"
            echo "  (tanpa flag)          → start backend + frontend production"
            echo "  --backend             → start backend (Rust) saja"
            echo "  --frontend            → start frontend production saja"
            echo "  --frontend --dev      → start frontend dev mode (Vite HMR)"
            echo "  --dev                 → start backend + frontend dev mode"
            exit 1
            ;;
    esac
done

# ── Resolve Bun binary ────────────────────────────────────────────────────────
resolve_bun() {
    if command -v bun &> /dev/null; then
        echo "bun"
    elif [ -x "$HOME/.bun/bin/bun" ]; then
        echo "$HOME/.bun/bin/bun"
    else
        echo ""
    fi
}

# ── Start Backend (Rust/Axum) ─────────────────────────────────────────────────
start_backend() {
    echo ""
    echo "=== [Backend] Rust/Axum — port 8080 ==="

    if ! command -v cargo &> /dev/null; then
        if [ -f "$HOME/.cargo/env" ]; then
            source "$HOME/.cargo/env"
        else
            echo "Error: 'cargo' tidak ditemukan. Pastikan Rust sudah terinstal."
            return 1
        fi
    fi

    cd "$APP_DIR" || return 1

    echo "Membangun aplikasi (Release mode)..."
    cargo build --release

    if [ $? -ne 0 ]; then
        echo "Error: Build gagal. Silakan periksa log di atas."
        echo ""
        echo "Pastikan semua build dependencies sudah terinstall:"
        echo "  Ubuntu/Debian: sudo apt install build-essential pkg-config libclang-dev libpam0g-dev"
        echo "  Arch/CachyOS:  sudo pacman -S base-devel clang pam"
        return 1
    fi

    echo "Build berhasil!"

    PID=$(pgrep -f "$BINARY_PATH")
    if [ -n "$PID" ]; then
        echo "Menghentikan instance backend yang sudah berjalan (PID: $PID)..."
        kill "$PID"
        sleep 2
    fi

    echo "Menjalankan backend..."
    nohup "$BINARY_PATH" > "$APP_DIR/server.log" 2>&1 &
    NEW_PID=$!

    echo "Backend berjalan di background (PID: $NEW_PID) — http://localhost:8080"
    echo "Log: $APP_DIR/server.log"

    sleep 1
    echo ""
    echo "--- Log Awal Backend ---"
    head -n 5 "$APP_DIR/server.log"
    echo "------------------------"
}

# ── Start Frontend Production (Bun + Hono serve static) ──────────────────────
start_frontend() {
    echo ""
    echo "=== [Frontend] Bun+Hono production — port 3000 ==="

    BUN=$(resolve_bun)
    if [ -z "$BUN" ]; then
        echo "Error: 'bun' tidak ditemukan."
        echo "Install Bun: curl -fsSL https://bun.sh/install | bash"
        return 1
    fi

    echo "Menggunakan Bun: $BUN"
    cd "$FRONTEND_DIR" || return 1

    if [ ! -d "node_modules/hono" ]; then
        echo "Menginstall dependencies..."
        "$BUN" install || return 1
    fi

    # Matikan Bun server lama jika ada
    PID=$(pgrep -f "bun.*server\.ts")
    if [ -n "$PID" ]; then
        echo "Menghentikan instance frontend yang sudah berjalan (PID: $PID)..."
        kill "$PID"
        sleep 1
    fi

    echo "Menjalankan frontend (production)..."
    nohup "$BUN" run server.ts > "$APP_DIR/frontend.log" 2>&1 &
    NEW_PID=$!

    echo "Frontend berjalan di background (PID: $NEW_PID) — http://localhost:3000"
    echo "Log: $APP_DIR/frontend.log"

    sleep 1
    echo ""
    echo "--- Log Awal Frontend ---"
    head -n 5 "$APP_DIR/frontend.log"
    echo "-------------------------"
}

# ── Start Frontend Dev (Vite HMR + Bun proxy) ────────────────────────────────
start_frontend_dev() {
    echo ""
    echo "=== [Frontend] Dev mode — Vite HMR (port 5173) + Bun proxy (port 3000) ==="

    BUN=$(resolve_bun)
    if [ -z "$BUN" ]; then
        echo "Error: 'bun' tidak ditemukan."
        echo "Install Bun: curl -fsSL https://bun.sh/install | bash"
        return 1
    fi

    echo "Menggunakan Bun: $BUN"
    cd "$FRONTEND_DIR" || return 1

    if [ ! -d "node_modules/hono" ]; then
        echo "Menginstall dependencies..."
        "$BUN" install || return 1
    fi

    # Matikan instance lama jika ada
    PID_BUN=$(pgrep -f "bun.*server\.ts")
    if [ -n "$PID_BUN" ]; then
        echo "Menghentikan Bun server lama (PID: $PID_BUN)..."
        kill "$PID_BUN"
        sleep 1
    fi

    PID_VITE=$(pgrep -f "vite.*5173")
    if [ -n "$PID_VITE" ]; then
        echo "Menghentikan Vite server lama (PID: $PID_VITE)..."
        kill "$PID_VITE"
        sleep 1
    fi

    # Jalankan Vite dev server di background (port 5173)
    echo "Menjalankan Vite dev server (port 5173)..."
    nohup "$BUN" run dev:vite > "$APP_DIR/vite.log" 2>&1 &
    VITE_PID=$!
    echo "Vite berjalan di background (PID: $VITE_PID) — http://localhost:5173"
    echo "Log: $APP_DIR/vite.log"

    # Tunggu Vite siap (max 10 detik)
    echo "Menunggu Vite siap..."
    for i in $(seq 1 10); do
        sleep 1
        if curl -sf http://localhost:5173 > /dev/null 2>&1; then
            echo "Vite siap!"
            break
        fi
        if [ $i -eq 10 ]; then
            echo "Peringatan: Vite belum merespons, lanjutkan anyway..."
        fi
    done

    # Jalankan Bun server tanpa NODE_ENV=production → IS_DEV=true → proxy ke Vite
    echo "Menjalankan Bun server dev mode (port 3000)..."
    nohup env NODE_ENV=development "$BUN" run server.ts > "$APP_DIR/frontend.log" 2>&1 &
    BUN_PID=$!

    echo "Bun server berjalan di background (PID: $BUN_PID) — http://localhost:3000"
    echo "Log: $APP_DIR/frontend.log"

    sleep 1
    echo ""
    echo "--- Log Awal Bun Server ---"
    head -n 5 "$APP_DIR/frontend.log"
    echo "---------------------------"
    echo ""
    echo "Dev mode aktif:"
    echo "  → Edit file di frontend-vue/src/ → browser auto-reload via Vite HMR"
    echo "  → Buka http://localhost:3000 (bukan 5173)"
}

# ── Main ──────────────────────────────────────────────────────────────────────
if [ "$DEV" = true ]; then
    # --dev tanpa --frontend → start backend juga
    if [ "$MODE" != "frontend" ]; then
        start_backend || exit 1
    fi
    start_frontend_dev
elif [ "$MODE" = "backend" ]; then
    start_backend
elif [ "$MODE" = "frontend" ]; then
    start_frontend
else
    # tanpa flag → production keduanya
    start_backend && start_frontend
fi
