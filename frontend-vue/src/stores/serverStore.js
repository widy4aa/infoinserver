import { ref, computed } from 'vue'
import { useStorage } from '@vueuse/core'

// State Global diletakkan di luar fungsi
const activeServerId = useStorage('active-server-id', '1')
const servers = useStorage('monitoring-servers', [
  { id: '1', name: 'Local Server', url: 'http://127.0.0.1:8080' }
])

// Multi-user token storage
// Struktur baru: { "server-1": { activeUser: "infratek", users: { "infratek": "token...", "webmaster": "token..." } } }
// Migrasi otomatis dari struktur lama: { "server-1": { token: "...", username: "infratek" } }
const serverTokens = useStorage('server-tokens', {}, sessionStorage)

// ── Migrasi otomatis dari struktur token lama ke baru ──────────────
const migrateTokenIfNeeded = (serverId) => {
  const data = serverTokens.value[serverId]
  if (!data) return
  // Deteksi struktur lama: punya field "token" dan "username" langsung
  if (data.token && data.username && !data.users) {
    const username = data.username
    const token = data.token
    serverTokens.value = {
      ...serverTokens.value,
      [serverId]: {
        activeUser: username,
        users: { [username]: token }
      }
    }
  }
}

export const useServerStore = () => {

  const getActiveServerUrl = () => {
    const server = servers.value.find(s => s.id === activeServerId.value)
    return server ? server.url : 'http://127.0.0.1:8080'
  }

  // ── Token management (Multi-user) ────────────────────────────
  const getActiveToken = (serverId) => {
    migrateTokenIfNeeded(serverId)
    const data = serverTokens.value[serverId]
    if (!data || !data.activeUser || !data.users) return null
    return data.users[data.activeUser] || null
  }

  // Untuk kompatibilitas dengan kode lama yang masih pakai getToken
  const getToken = (serverId) => getActiveToken(serverId)

  const getActiveUsername = (serverId) => {
    migrateTokenIfNeeded(serverId)
    return serverTokens.value[serverId]?.activeUser || null
  }

  // Untuk kompatibilitas dengan kode lama
  const getUsername = (serverId) => getActiveUsername(serverId)

  const listServerUsers = (serverId) => {
    migrateTokenIfNeeded(serverId)
    const data = serverTokens.value[serverId]
    if (!data || !data.users) return []
    return Object.keys(data.users)
  }

  const addUserToken = (serverId, username, token) => {
    migrateTokenIfNeeded(serverId)
    const existing = serverTokens.value[serverId] || { activeUser: username, users: {} }
    serverTokens.value = {
      ...serverTokens.value,
      [serverId]: {
        activeUser: existing.activeUser || username,
        users: {
          ...(existing.users || {}),
          [username]: token
        }
      }
    }
  }

  const switchUser = (serverId, username) => {
    migrateTokenIfNeeded(serverId)
    const data = serverTokens.value[serverId]
    if (!data || !data.users || !data.users[username]) return false
    serverTokens.value = {
      ...serverTokens.value,
      [serverId]: {
        ...data,
        activeUser: username
      }
    }
    return true
  }

  const removeUser = (serverId, username) => {
    migrateTokenIfNeeded(serverId)
    const data = serverTokens.value[serverId]
    if (!data || !data.users) return
    const newUsers = { ...data.users }
    delete newUsers[username]
    const remaining = Object.keys(newUsers)
    const newActiveUser = data.activeUser === username
      ? (remaining[0] || null)
      : data.activeUser
    if (!newActiveUser) {
      // Tidak ada user tersisa, hapus seluruh entry server
      const tokens = { ...serverTokens.value }
      delete tokens[serverId]
      serverTokens.value = tokens
    } else {
      serverTokens.value = {
        ...serverTokens.value,
        [serverId]: {
          activeUser: newActiveUser,
          users: newUsers
        }
      }
    }
  }

  // setToken sekarang menggunakan addUserToken + set sebagai activeUser
  const setToken = (serverId, token, username) => {
    addUserToken(serverId, username, token)
    // Set sebagai active user juga
    const data = serverTokens.value[serverId]
    if (data) {
      serverTokens.value = {
        ...serverTokens.value,
        [serverId]: {
          ...data,
          activeUser: username
        }
      }
    }
  }

  const clearToken = (serverId) => {
    const tokens = { ...serverTokens.value }
    delete tokens[serverId]
    serverTokens.value = tokens
  }

  const clearAllTokens = () => {
    serverTokens.value = {}
  }

  const isAuthenticated = (serverId) => !!getActiveToken(serverId)

  // ── Server CRUD ──────────────────────────────────────────
  const addServer = (name, url, customId) => {
    const cleanUrl = url.endsWith('/') ? url.slice(0, -1) : url
    servers.value.push({
      id: customId || Date.now().toString(),
      name,
      url: cleanUrl
    })
  }

  const removeServer = (id) => {
    servers.value = servers.value.filter(s => s.id !== id)
    clearToken(id)
    if (activeServerId.value === id && servers.value.length > 0) {
      activeServerId.value = servers.value[0].id
    }
  }

  const setActiveServer = (id) => {
    activeServerId.value = id
  }

  const updateServerName = (id, newName) => {
    const server = servers.value.find(s => s.id === id)
    if (server) server.name = newName
  }

  // Simpan os_name yang terdeteksi dari backend ke localStorage
  const setServerOsName = (id, osName) => {
    if (!osName) return
    const server = servers.value.find(s => s.id === id)
    if (server && server.os_name !== osName) {
      server.os_name = osName
    }
  }

  return {
    servers,
    activeServerId,
    getActiveServerUrl,
    // Token management
    getToken,          // kompatibilitas kode lama
    getActiveToken,    // baru
    getActiveUsername, // baru
    getUsername,       // kompatibilitas kode lama
    setToken,
    addUserToken,      // baru
    switchUser,        // baru
    removeUser,        // baru
    listServerUsers,   // baru
    clearToken,
    clearAllTokens,
    isAuthenticated,
    // Server CRUD
    addServer,
    removeServer,
    setActiveServer,
    updateServerName,
    setServerOsName,
  }
}
