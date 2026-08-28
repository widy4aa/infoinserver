// src/composables/useApi.js
// Wrapper fetch yang otomatis inject Authorization: Bearer token per server
// Jika response 401 → clear token → emit event agar ServerLayout tampilkan login modal

import { useServerStore } from '../stores/serverStore'

export const useApi = () => {
  const { getToken, activeServerId, clearToken } = useServerStore()

  /**
   * apiFetch — drop-in replacement untuk fetch()
   * Otomatis tambah Authorization header dari token server aktif
   * Jika 401 → hapus token agar login modal muncul kembali
   */
  const apiFetch = async (url, options = {}) => {
    const token = getToken(activeServerId.value)

    const headers = {
      ...(options.headers || {}),
    }
    if (token) {
      headers['Authorization'] = `Bearer ${token}`
    }

    const res = await fetch(url, { ...options, headers })

    // Jika 401 — token expired atau invalid, clear token
    if (res.status === 401) {
      clearToken(activeServerId.value)
      // Dispatch event agar ServerLayout tahu harus tampilkan login modal
      window.dispatchEvent(new CustomEvent('auth:expired', {
        detail: { serverId: activeServerId.value }
      }))
    }

    return res
  }

  return { apiFetch }
}
