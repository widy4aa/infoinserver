// src/composables/useApi.js
// Wrapper fetch yang otomatis inject Authorization: Bearer token per server
// Menggunakan getActiveToken() untuk mendukung multi-user session

import { useServerStore } from '../stores/serverStore'
import { useRouter } from 'vue-router'
import { useToastStore } from '../stores/toastStore'

export const useApi = () => {
  const { getActiveToken, activeServerId, removeUser, getActiveUsername, listServerUsers } = useServerStore()
  const router = useRouter()
  const toastStore = useToastStore()

  const apiFetch = async (url, options = {}) => {
    // Gunakan getActiveToken agar mendukung multi-user switch
    const token = getActiveToken(activeServerId.value)

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
      const currentUser = getActiveUsername(activeServerId.value)
      const serverId = activeServerId.value

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
        // Tidak ada user tersisa, redirect ke homepage untuk login ulang
        if (router) {
          router.push('/')
        } else {
          window.location.href = '/'
        }
      }
      // Jika masih ada user lain, tinggalkan agar komponen re-render otomatis

      return Promise.reject(new Error("Authentication failed"))
    }

    return res
  }

  return { apiFetch }
}

