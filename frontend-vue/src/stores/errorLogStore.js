/**
 * errorLogStore.js
 *
 * Store in-memory untuk error yang terjadi di sisi browser (client-side).
 * Error disimpan dalam circular buffer max 500 entries — LIFO (terbaru di atas).
 *
 * Dua fungsi utama:
 *   1. pushError(entry)   — tambah error ke buffer in-memory (real-time di UI)
 *   2. sendErrors(batch)  — kirim batch ke Bun /api/client-errors (persisten di SQLite)
 *
 * sendErrors dipanggil fire-and-forget dari useApi.js setiap kali apiFetch
 * mendapat response dengan status >= 400.
 *
 * Kenapa dua layer?
 *   - In-memory: real-time tanpa fetch, langsung kelihatan di UI saat error terjadi
 *   - SQLite (via Bun): persisten, bisa dilihat setelah refresh / dari session lain
 */

import { ref } from 'vue'
import { useAuthStore } from './authStore'

const MAX_ENTRIES = 500

// In-memory circular buffer — reactive, langsung update UI
const errors = ref([])

// Flag untuk cegah infinite loop saat sendErrors itu sendiri gagal
let _sending = false

/**
 * Tambah satu error entry ke buffer in-memory.
 * @param {{ server_id, server_name, method, path, status, message, level }} entry
 */
const pushError = (entry) => {
  const normalized = {
    id:          Date.now() + Math.random(), // client-side id unik
    timestamp:   new Date().toISOString(),
    server_id:   entry.server_id   ?? null,
    server_name: entry.server_name ?? null,
    method:      entry.method      ?? null,
    path:        entry.path        ?? '(unknown)',
    status:      entry.status      ?? null,
    message:     entry.message     ?? null,
    level:       entry.level       ?? 'ERROR',
  }
  errors.value.unshift(normalized)
  // Potong jika melebihi batas
  if (errors.value.length > MAX_ENTRIES) {
    errors.value = errors.value.slice(0, MAX_ENTRIES)
  }
}

/**
 * Kirim batch error ke Bun /api/client-errors (fire-and-forget).
 * Tidak throw — kegagalan silent agar tidak mengganggu alur utama.
 * @param {Array} batch
 */
const sendErrors = async (batch) => {
  if (_sending || !batch || batch.length === 0) return
  _sending = true
  try {
    // Ambil GitHub session token dari authStore untuk auth
    const { getToken } = useAuthStore()
    const token = getToken()
    if (!token) return // belum login, skip

    await fetch('/api/client-errors', {
      method:  'POST',
      headers: {
        'Content-Type':  'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify({ errors: batch }),
    })
  } catch {
    // Silent fail — jangan sampai error logging menyebabkan error baru
  } finally {
    _sending = false
  }
}

/**
 * Hapus semua error dari buffer in-memory.
 */
const clearErrors = () => { errors.value = [] }

export const useErrorLogStore = () => ({
  errors,
  pushError,
  sendErrors,
  clearErrors,
})
