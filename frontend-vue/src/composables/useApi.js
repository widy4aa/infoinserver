// src/composables/useApi.js
// Wrapper fetch yang otomatis inject Authorization: Bearer token per server
// Menggunakan getActiveToken() untuk mendukung multi-user session
// Auto-refresh token proaktif jika token akan expire dalam 5 menit

import { useServerStore } from '../stores/serverStore'
import { useRouter } from 'vue-router'
import { useToastStore } from '../stores/toastStore'

// ── Token expiry helpers ──────────────────────────────────────────────────────

/**
 * Decode JWT payload tanpa verifikasi signature (hanya baca exp claim)
 * @param {string} token
 * @returns {object|null}
 */
export const decodeJwtPayload = (token) => {
  try {
    const base64Payload = token.split('.')[1]
    if (!base64Payload) return null
    const json = atob(base64Payload.replace(/-/g, '+').replace(/_/g, '/'))
    return JSON.parse(json)
  } catch {
    return null
  }
}

/**
 * Cek apakah token sudah/akan expire
 * @param {string} token
 * @param {number} bufferSeconds — anggap expire jika sisa waktu < buffer (default 5 menit)
 * @returns {boolean}
 */
export const isTokenExpired = (token, bufferSeconds = 300) => {
  if (!token) return true
  const payload = decodeJwtPayload(token)
  if (!payload?.exp) return true
  const nowSecs = Math.floor(Date.now() / 1000)
  return payload.exp - nowSecs < bufferSeconds
}

// ── Refresh lock — cegah multiple concurrent refresh calls ───────────────────
let _refreshPromise = null

/**
 * Refresh token via POST /api/auth/refresh
 * Return token baru jika berhasil, null jika gagal
 */
const refreshToken = async (serverUrl, currentToken) => {
  if (_refreshPromise) return _refreshPromise

  _refreshPromise = fetch(`${serverUrl}/api/auth/refresh`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token: currentToken }),
  })
    .then(async (res) => {
      if (!res.ok) return null
      const data = await res.json()
      return data.token || null
    })
    .catch(() => null)
    .finally(() => { _refreshPromise = null })

  return _refreshPromise
}

// ── Main composable ───────────────────────────────────────────────────────────

export const useApi = () => {
  const { getActiveToken, getActiveServerUrl, activeServerId, removeUser, getActiveUsername, listServerUsers, setToken } = useServerStore()
  const router = useRouter()
  const toastStore = useToastStore()

  const apiFetch = async (url, options = {}) => {
    const serverId = activeServerId.value
    let token = getActiveToken(serverId)

    // ── Proaktif refresh jika token akan expire dalam 5 menit ──
    if (token && isTokenExpired(token, 300)) {
      const serverUrl = getActiveServerUrl()
      const newToken = await refreshToken(serverUrl, token)
      if (newToken) {
        const username = getActiveUsername(serverId)
        if (username) setToken(serverId, newToken, username)
        token = newToken
      }
      // Jika refresh gagal, lanjutkan dengan token lama — backend akan return 401
      // dan ditangani di bawah
    }

    const headers = {
      ...(options.headers || {}),
    }
    if (token) {
      headers['Authorization'] = `Bearer ${token}`
    }

    let res = await fetch(url, { ...options, headers })

    // Cek jika status 401 secara resmi
    let isAuthFailed = res.status === 401

    // Cek jika status 500 dari backend namun berisi pesan sudo "Authentication failed"
    if (!isAuthFailed && res.status === 500) {
      const clonedRes = res.clone()
      try {
        const errText = await clonedRes.text()
        if (errText.toLowerCase().includes('sudo: authentication failed')) {
          isAuthFailed = true
        }
      } catch (e) {}
    }

    if (isAuthFailed) {
      const currentUser = getActiveUsername(serverId)

      // Hapus token user yang bermasalah dari daftar
      if (currentUser) {
        removeUser(serverId, currentUser)
      }

      // Cek apakah masih ada user lain yang bisa dipakai
      const remainingUsers = listServerUsers(serverId)

      if (toastStore) {
        if (remainingUsers.length > 0) {
          toastStore.showToast("Session Expired", `Session for "${currentUser}" expired. Switched to "${remainingUsers[0]}".`, "warning")
        } else {
          toastStore.showToast("Session Expired", "Authentication failed. Please login again.", "error")
        }
      }

      if (remainingUsers.length === 0) {
        // Dispatch event agar ServerLayout menampilkan LoginModal di tempat
        // tanpa navigasi keluar dari halaman server saat ini
        window.dispatchEvent(new CustomEvent('auth:expired', { detail: { serverId } }))
      }
      // Jika masih ada user lain, tinggalkan agar komponen re-render otomatis

      return Promise.reject(new Error("Authentication failed"))
    }

    return res
  }

  return { apiFetch }
}
