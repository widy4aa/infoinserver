import { ref, watch } from 'vue'
import { useStorage } from '@vueuse/core'

// ── State Global ──────────────────────────────────────────────────────────────
const activeServerId = useStorage('active-server-id', '1')
const servers = useStorage('monitoring-servers', [
  { id: '1', name: 'Local Server', url: 'http://127.0.0.1:3000' }
])

// ── Labels: { id, name, serverIds[] } ────────────────────────────────────────
const labels = useStorage('server-labels', [])

// ── Migrasi URL lama port 8080 → 3000 ────────────────────────────────────────
const migrateServerUrls = () => {
  servers.value = servers.value.map(s => {
    if (s.url && s.url.includes(':8080')) {
      return { ...s, url: s.url.replace(':8080', ':3000') }
    }
    return s
  })
}
migrateServerUrls()

// ── Multi-user token storage ──────────────────────────────────────────────────
// Sengaja di sessionStorage — menyimpan password Linux, tidak boleh di-sync
const serverTokens = useStorage('server-tokens', {}, sessionStorage)

const migrateTokenIfNeeded = (serverId) => {
  const data = serverTokens.value[serverId]
  if (!data) return
  if (data.token && data.username && !data.users) {
    serverTokens.value = {
      ...serverTokens.value,
      [serverId]: { activeUser: data.username, users: { [data.username]: data.token } }
    }
  }
}

// ── Debounce helper ───────────────────────────────────────────────────────────
const debounce = (fn, delay) => {
  let timer = null
  return (...args) => {
    clearTimeout(timer)
    timer = setTimeout(() => fn(...args), delay)
  }
}

// ── Config sync state ─────────────────────────────────────────────────────────
let _githubToken = null          // di-set dari luar via setGithubToken()
let _syncEnabled = false         // aktif setelah loadConfigFromServer() selesai
let _isSyncing   = false         // cegah watch loop saat load

// Dipanggil dari App.vue saat user login GitHub
const setGithubToken = (token) => { _githubToken = token }

// GET config dari Bun SQLite → replace localStorage
const loadConfigFromServer = async (token) => {
  if (!token) return
  _githubToken = token
  try {
    const res = await fetch('/api/frontend/config', {
      headers: { Authorization: `Bearer ${token}` }
    })
    if (!res.ok) return

    const data = await res.json()

    // exists: false → user baru, tidak ada data di server
    // Dalam hal ini push localStorage saat ini ke server agar tersimpan
    if (!data.exists) {
      _syncEnabled = true
      saveConfigToServer()
      return
    }

    // Suspend watch sementara agar tidak trigger save saat kita load
    _isSyncing = true
    servers.value          = data.servers          ?? []
    labels.value           = data.labels           ?? []
    activeServerId.value   = data.active_server_id ?? servers.value[0]?.id ?? ''
    // Beri waktu Vue flush reactive update sebelum re-enable watch
    setTimeout(() => {
      _isSyncing   = false
      _syncEnabled = true
    }, 100)

  } catch (e) {
    console.warn('[config] Failed to load from server, using localStorage:', e)
    _syncEnabled = true
  }
}

// PUT config ke Bun SQLite (debounced 1.5 detik)
const saveConfigToServer = debounce(async () => {
  if (!_githubToken || !_syncEnabled || _isSyncing) return
  try {
    await fetch('/api/frontend/config', {
      method: 'PUT',
      headers: {
        'Content-Type':  'application/json',
        'Authorization': `Bearer ${_githubToken}`
      },
      body: JSON.stringify({
        servers:          servers.value,
        labels:           labels.value,
        active_server_id: activeServerId.value,
      })
    })
  } catch (e) {
    console.warn('[config] Failed to save config to server:', e)
  }
}, 1500)

// Auto-save saat ada perubahan
watch([servers, labels, activeServerId], () => {
  if (_syncEnabled && !_isSyncing) saveConfigToServer()
}, { deep: true })

// ── Store ─────────────────────────────────────────────────────────────────────
export const useServerStore = () => {

  const getActiveServerUrl = () => {
    const server = servers.value.find(s => s.id === activeServerId.value)
    return server ? server.url : 'http://127.0.0.1:3000'
  }

  // ── Token management ──────────────────────────────────────────────────────
  const getActiveToken = (serverId) => {
    migrateTokenIfNeeded(serverId)
    const data = serverTokens.value[serverId]
    if (!data || !data.activeUser || !data.users) return null
    return data.users[data.activeUser] || null
  }

  const getToken = (serverId) => getActiveToken(serverId)

  const getActiveUsername = (serverId) => {
    migrateTokenIfNeeded(serverId)
    return serverTokens.value[serverId]?.activeUser || null
  }

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
        users: { ...(existing.users || {}), [username]: token }
      }
    }
  }

  const switchUser = (serverId, username) => {
    migrateTokenIfNeeded(serverId)
    const data = serverTokens.value[serverId]
    if (!data || !data.users || !data.users[username]) return false
    serverTokens.value = {
      ...serverTokens.value,
      [serverId]: { ...data, activeUser: username }
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
    const newActiveUser = data.activeUser === username ? (remaining[0] || null) : data.activeUser
    if (!newActiveUser) {
      const tokens = { ...serverTokens.value }
      delete tokens[serverId]
      serverTokens.value = tokens
    } else {
      serverTokens.value = {
        ...serverTokens.value,
        [serverId]: { activeUser: newActiveUser, users: newUsers }
      }
    }
  }

  const setToken = (serverId, token, username) => {
    addUserToken(serverId, username, token)
    const data = serverTokens.value[serverId]
    if (data) {
      serverTokens.value = {
        ...serverTokens.value,
        [serverId]: { ...data, activeUser: username }
      }
    }
  }

  const clearToken = (serverId) => {
    const tokens = { ...serverTokens.value }
    delete tokens[serverId]
    serverTokens.value = tokens
  }

  const clearAllTokens = () => { serverTokens.value = {} }

  const isAuthenticated = (serverId) => !!getActiveToken(serverId)

  // ── Server CRUD ───────────────────────────────────────────────────────────
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
    labels.value = labels.value.map(l => ({
      ...l,
      serverIds: l.serverIds.filter(sid => sid !== id)
    }))
  }

  const setActiveServer = (id) => { activeServerId.value = id }

  const updateServerName = (id, newName) => {
    const server = servers.value.find(s => s.id === id)
    if (server) server.name = newName
  }

  const setServerOsName = (id, osName) => {
    if (!osName) return
    const server = servers.value.find(s => s.id === id)
    if (server && server.os_name !== osName) server.os_name = osName
  }

  // ── Label CRUD ────────────────────────────────────────────────────────────
  const addLabel = (name) => {
    labels.value.push({
      id: 'lbl_' + Date.now(),
      name: name || 'New Label',
      serverIds: []
    })
  }

  const removeLabel = (id) => {
    labels.value = labels.value.filter(l => l.id !== id)
  }

  const renameLabel = (id, name) => {
    const label = labels.value.find(l => l.id === id)
    if (label) label.name = name
  }

  const assignServerToLabel = (serverId, labelId, insertIdx = null) => {
    labels.value = labels.value.map(l => ({
      ...l,
      serverIds: l.serverIds.filter(sid => sid !== serverId)
    }))
    if (labelId === null) return
    const label = labels.value.find(l => l.id === labelId)
    if (!label) return
    if (insertIdx !== null && insertIdx >= 0) {
      label.serverIds.splice(insertIdx, 0, serverId)
    } else {
      label.serverIds.push(serverId)
    }
  }

  const reorderServersInLabel = (labelId, newIds) => {
    const label = labels.value.find(l => l.id === labelId)
    if (label) label.serverIds = newIds
  }

  const reorderLabels = (newOrder) => { labels.value = newOrder }

  const getServerLabel = (serverId) => {
    return labels.value.find(l => l.serverIds.includes(serverId)) || null
  }

  const ungroupedServers = () => {
    const allLabeled = new Set(labels.value.flatMap(l => l.serverIds))
    return servers.value.filter(s => !allLabeled.has(s.id))
  }

  return {
    servers,
    activeServerId,
    labels,
    getActiveServerUrl,
    // Token management
    getToken,
    getActiveToken,
    getActiveUsername,
    getUsername,
    setToken,
    addUserToken,
    switchUser,
    removeUser,
    listServerUsers,
    clearToken,
    clearAllTokens,
    isAuthenticated,
    // Server CRUD
    addServer,
    removeServer,
    setActiveServer,
    updateServerName,
    setServerOsName,
    // Label CRUD
    addLabel,
    removeLabel,
    renameLabel,
    assignServerToLabel,
    reorderServersInLabel,
    reorderLabels,
    getServerLabel,
    ungroupedServers,
    // Config sync
    loadConfigFromServer,
    setGithubToken,
  }
}
