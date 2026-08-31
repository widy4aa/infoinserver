// src/composables/useApi.js
// Wrapper fetch yang otomatis inject Authorization: Bearer token per server
// Jika response 401 atau mengandung error autentikasi sudo → clear token → kembali ke halaman utama

import { useServerStore } from '../stores/serverStore'
import { useRouter } from 'vue-router'
import { useToastStore } from '../stores/toastStore'

export const useApi = () => {
  const { getToken, activeServerId, clearToken } = useServerStore()
  const router = useRouter()
  const toastStore = useToastStore()

  /**
   * apiFetch — drop-in replacement untuk fetch()
   * Otomatis tambah Authorization header dari token server aktif
   * Jika 401 atau error sudo auth failed → hapus token & kick ke homepage
   */
  const apiFetch = async (url, options = {}) => {
    const token = getToken(activeServerId.value)

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
      // Hapus token
      clearToken(activeServerId.value)
      
      // Tampilkan toast peringatan
      if (toastStore) {
        toastStore.showToast("Session Expired", "Server authentication failed. Please login again.", "error")
      }

      // Paksa kembali ke Homepage (Root /)
      if (router) {
        router.push('/')
      } else {
        // Fallback jika router gagal di-load di dalam composable
        window.location.href = '/'
      }
      
      // Ubah status response agar pemanggil (.catch di view) berhenti melanjutkan proses
      return Promise.reject(new Error("Authentication failed (Sudo/JWT)"))
    }

    return res
  }

  return { apiFetch }
}
