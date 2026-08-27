import { reactive } from 'vue'
import { useStorage } from '@vueuse/core'

export const useServerStore = () => {
  // Simpan list server di localStorage browser
  const servers = useStorage('monitoring-servers', [
    { id: '1', name: 'Local Server', url: 'http://127.0.0.1:8080' }
  ])
  
  // Server mana yang sedang aktif
  const activeServerId = useStorage('active-server-id', '1')

  const getActiveServerUrl = () => {
    const server = servers.value.find(s => s.id === activeServerId.value)
    return server ? server.url : 'http://127.0.0.1:8080'
  }

  const addServer = (name, url) => {
    // Bersihkan trailing slash
    const cleanUrl = url.endsWith('/') ? url.slice(0, -1) : url;
    servers.value.push({
      id: Date.now().toString(),
      name,
      url: cleanUrl
    })
  }

  const removeServer = (id) => {
    servers.value = servers.value.filter(s => s.id !== id)
    if (activeServerId.value === id && servers.value.length > 0) {
      activeServerId.value = servers.value[0].id
    }
  }

  const setActiveServer = (id) => {
    activeServerId.value = id
  }

  const updateServerName = (id, newName) => {
    const server = servers.value.find(s => s.id === id)
    if (server) {
      server.name = newName
    }
  }

  return {
    servers,
    activeServerId,
    getActiveServerUrl,
    addServer,
    removeServer,
    setActiveServer,
    updateServerName
  }
}
