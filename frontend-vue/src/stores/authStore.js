// src/stores/authStore.js
// Menyimpan sesi GitHub OAuth (terpisah dari JWT Linux per-server)

import { ref, computed } from 'vue'

const SESSION_KEY = 'github-session'

const readSession = () => {
  try {
    const raw = localStorage.getItem(SESSION_KEY)
    if (!raw || raw === 'null') return null
    return JSON.parse(raw)
  } catch { return null }
}

const githubSession = ref(readSession())

export const isLoggedInSync = () => !!readSession()?.token

export const useAuthStore = () => {
  const isLoggedIn = computed(() => !!githubSession.value?.token)

  const githubUser = computed(() => {
    if (!githubSession.value) return null
    return {
      username:         githubSession.value.username,
      name:             githubSession.value.name,
      avatar:           githubSession.value.avatar,
      role:             githubSession.value.role             ?? 'slave',
      restrictedLabels: githubSession.value.restrictedLabels ?? [],
      allowedLabels:    githubSession.value.allowedLabels    ?? null, // null = akses semua
      canWrite:         githubSession.value.canWrite         ?? false,
    }
  })

  // Role helpers
  const isAdmin  = computed(() => githubSession.value?.role === 'admin')
  const isMaster = computed(() => githubSession.value?.role === 'master')
  // isSlave: built-in slave atau custom role yang tidak bisa write
  const isSlave  = computed(() => {
    const role = githubSession.value?.role ?? 'slave'
    if (role === 'slave') return true
    if (role === 'admin' || role === 'master') return false
    // Custom role: bukan slave jika bisa write (tapi masih punya restriction)
    return false
  })

  const userRole         = computed(() => githubSession.value?.role ?? 'slave')
  const restrictedLabels = computed(() => githubSession.value?.restrictedLabels ?? [])
  const allowedLabels    = computed(() => githubSession.value?.allowedLabels ?? null)
  // canWriteConfig: dari server (canWrite field) atau dari built-in logic
  const canWriteConfig   = computed(() => {
    const role = githubSession.value?.role ?? 'slave'
    if (role === 'admin' || role === 'master') return true
    if (role === 'slave') return false
    return githubSession.value?.canWrite ?? false
  })

  const setSession = ({ token, username, name, avatar, role = 'slave', restrictedLabels = [], allowedLabels = null, canWrite = false }) => {
    const data = { token, username, name, avatar, role, restrictedLabels, allowedLabels, canWrite }
    localStorage.setItem(SESSION_KEY, JSON.stringify(data))
    githubSession.value = data
  }

  // Fetch role dari server dan update session
  const fetchAndUpdateRole = async () => {
    const token = githubSession.value?.token
    if (!token) return
    try {
      const res = await fetch('/api/roles/me', {
        headers: { Authorization: `Bearer ${token}` }
      })
      if (!res.ok) return
      const data = await res.json()
      const updated = {
        ...githubSession.value,
        role:             data.role             ?? 'slave',
        restrictedLabels: data.restrictedLabels ?? [],
        allowedLabels:    data.allowedLabels    ?? null,
        canWrite:         data.canWrite         ?? false,
      }
      localStorage.setItem(SESSION_KEY, JSON.stringify(updated))
      githubSession.value = updated
    } catch (e) {
      console.warn('[authStore] Failed to fetch role:', e)
    }
  }

  const logout = () => {
    localStorage.removeItem(SESSION_KEY)
    githubSession.value = null
  }

  const getToken = () => githubSession.value?.token || null

  return {
    isLoggedIn,
    githubUser,
    isAdmin,
    isMaster,
    isSlave,
    userRole,
    restrictedLabels,
    allowedLabels,
    canWriteConfig,
    setSession,
    fetchAndUpdateRole,
    logout,
    getToken,
  }
}
