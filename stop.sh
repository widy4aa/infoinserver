#!/bin/bash

# stop.sh - Script untuk menghentikan InfoIn Server Dashboard
# Usage:
#   ./stop.sh             → stop backend + frontend
#   ./stop.sh --backend   → stop backend (Rust) saja
#   ./stop.sh --frontend  → stop frontend (Bun+Hono) saja

APP_NAME="infoinserver"
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="$APP_DIR/target/release/$APP_NAME"

# ── Stop Backend ──────────────────────────────────────────────────────────────
stop_backend() {
    echo ""
    echo "=== [Backend] Menghentikan Rust/Axum ==="

    PID=$(pgrep -f "$BINARY_PATH")

    if [ -z "$PID" ]; then
        echo "Backend tidak sedang berjalan."
        return 0
    fi

    echo "Ditemukan proses backend (PID: $PID), menghentikan..."
    kill "$PID"

    if [ $? -eq 0 ]; then
        echo "Backend berhasil dihentikan."
    else
        echo "Gagal menghentikan backend. Coba: kill -9 $PID"
    fi
}

# ── Stop Frontend ─────────────────────────────────────────────────────────────
stop_frontend() {
    echo ""
    echo "=== [Frontend] Menghentikan Bun+Hono ==="

    PID=$(pgrep -f "bun.*server\.ts")

    if [ -z "$PID" ]; then
        echo "Frontend tidak sedang berjalan."
    else
        echo "Ditemukan proses frontend (PID: $PID), menghentikan..."
        kill "$PID" && echo "Frontend berhasil dihentikan." || echo "Gagal. Coba: kill -9 $PID"
    fi

    # Stop Vite dev server juga jika ada
    PID_VITE=$(pgrep -f "vite.*5173")
    if [ -n "$PID_VITE" ]; then
        echo "Ditemukan Vite dev server (PID: $PID_VITE), menghentikan..."
        kill "$PID_VITE" && echo "Vite berhasil dihentikan." || echo "Gagal. Coba: kill -9 $PID_VITE"
    fi
}

# ── Main ──────────────────────────────────────────────────────────────────────
case "${1:-}" in
    --backend)
        stop_backend
        ;;
    --frontend)
        stop_frontend
        ;;
    "")
        stop_backend
        stop_frontend
        ;;
    *)
        echo "Usage: $0 [--backend | --frontend]"
        echo "  (tanpa flag) → stop backend + frontend"
        exit 1
        ;;
esac
