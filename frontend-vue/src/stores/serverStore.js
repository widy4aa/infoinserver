import { ref } from 'vue'
import { useStorage } from '@vueuse/core'

// State Global diletakkan di luar fungsi
const activeServerId = useStorage('active-server-id', '1')
const servers = useStorage('monitoring-servers', [
  { id: '1', name: 'Local Server', url: 'http://127.0.0.1:8080' }
])
// Token ditaruh di luar fungsi sebagai singleton
const serverTokens = useStorage('server-tokens', {}, sessionStorage)

export const useServerStore = () => {

  const getActiveServerUrl = () => {
    const server = servers.value.find(s => s.id === activeServerId.value)
    return server ? server.url : 'http://127.0.0.1:8080'
  }

  // ── Token management ─────────────────────────────────────
  const getToken = (serverId) => serverTokens.value[serverId]?.token || null

  const setToken = (serverId, token, username) => {
    serverTokens.value = {
      ...serverTokens.value,
      [serverId]: { token, username }
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

  const isAuthenticated = (serverId) => !!getToken(serverId)

  const getUsername = (serverId) => serverTokens.value[serverId]?.username || null

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
    getToken,
    setToken,
    clearToken,
    clearAllTokens,
    isAuthenticated,
    getUsername,
    addServer,
    removeServer,
    setActiveServer,
    updateServerName,
    setServerOsName,
  }
}
