<script setup>
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import {
  FolderTree, DownloadCloud, Upload, ArrowUp, Folder, FileText, Download,
  X, Save, Trash2, Archive, FileEdit, Move, Copy, Info, HardDrive, Usb,
  RefreshCw, Lock, ChevronRight, Home, AlertTriangle, Loader2,
  Search, List, LayoutGrid, AlignJustify
} from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl, getToken, activeServerId } = useServerStore()
const { showToast, showConfirm } = useToastStore()
const { isDark } = useThemeStore()

// ── STATE ──
const files = ref([])
const currentPath = ref('/')
const homeRoot = ref('/')
const disks = ref([])
const isLoadingFiles = ref(false)
const isLoadingDisks = ref(false)
const pathInput = ref('/')

// ── SEARCH STATE ──
const searchQuery = ref('')
const searchResults = ref([])
const isSearchMode = ref(false)
const isSearching = ref(false)
let searchDebounceTimer = null

// ── VIEW MODE ──
const viewMode = ref(localStorage.getItem('files-view-mode') || 'list')
watch(viewMode, (val) => localStorage.setItem('files-view-mode', val))

// ── IMAGE HELPERS ──
const IMAGE_EXTS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg']
const isImage = (f) => {
  const ext = f.name.split('.').pop()?.toLowerCase()
  return IMAGE_EXTS.includes(ext)
}
const thumbnailUrl = (fullPathOrName, isFullPath = false) => {
  const path = isFullPath ? fullPathOrName : getFullPath(fullPathOrName)
  const token = getToken(activeServerId.value)
  return `${getActiveServerUrl()}/api/files/download?path=${encodeURIComponent(path)}&token=${token}`
}

// ── COMPUTED: unified display list ──
const displayFiles = computed(() => {
  if (isSearchMode.value) return searchResults.value
  // Local filter saat user mengetik di search bar tapi belum tekan Enter/Search
  if (searchQuery.value.trim() && !isSearchMode.value) {
    const q = searchQuery.value.toLowerCase()
    return files.value.filter(f => f.name.toLowerCase().includes(q))
  }
  return files.value
})

// ── COMPUTED ──
const breadcrumbs = computed(() => {
  const parts = currentPath.value.split('/').filter(p => p !== '')
  const crumbs = [{ label: '/', path: '/' }]
  let cumPath = ''
  for (const p of parts) {
    cumPath += '/' + p
    crumbs.push({ label: p, path: cumPath })
  }
  return crumbs
})

const isReadOnly = computed(() => {
  // Read-only jika bukan di dalam home_root dan bukan di removable mounts
  const path = currentPath.value
  const homeBase = homeRoot.value

  if (path === homeBase || path.startsWith(homeBase + '/')) return false

  // Cek apakah di dalam removable mount points
  for (const disk of disks.value) {
    if (disk.children) {
      for (const part of disk.children) {
        if (part.mountpoint && (path === part.mountpoint || path.startsWith(part.mountpoint + '/'))) {
          return !disk.is_removable
        }
      }
    }
    if (disk.mountpoint && (path === disk.mountpoint || path.startsWith(disk.mountpoint + '/'))) {
      return !disk.is_removable
    }
  }
  return true
})

// ── FETCH FILES ──
const fetchFiles = async (path) => {
  isLoadingFiles.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/list?path=${encodeURIComponent(path)}`)
    if (!res.ok) {
      const err = await res.text()
      showToast("Error", err, "error")
      return
    }
    files.value = await res.json()
    currentPath.value = path
    pathInput.value = path
  } catch (e) {
    showToast("Error", "Failed to list directory", "error")
  } finally {
    isLoadingFiles.value = false
  }
}

const navigateTo = (path) => fetchFiles(path)

const navigateUp = () => {
  if (currentPath.value === '/') return
  const parts = currentPath.value.split('/').filter(p => p !== '')
  parts.pop()
  fetchFiles('/' + parts.join('/'))
}

const navigateHome = () => fetchFiles(homeRoot.value)

const onPathInputEnter = () => fetchFiles(pathInput.value)

// ── SEARCH FUNCTIONS ──
const triggerSearch = async () => {
  const q = searchQuery.value.trim()
  if (!q || q.length < 2) {
    clearSearch()
    return
  }
  isSearching.value = true
  isSearchMode.value = true
  searchResults.value = []
  try {
    const res = await apiFetch(
      `${getActiveServerUrl()}/api/files/search?path=${encodeURIComponent(currentPath.value)}&query=${encodeURIComponent(q)}`
    )
    if (res.ok) {
      searchResults.value = await res.json()
    } else {
      showToast("Error", await res.text(), "error")
    }
  } catch (e) {
    showToast("Error", "Search failed", "error")
  } finally {
    isSearching.value = false
  }
}

const clearSearch = () => {
  searchQuery.value = ''
  searchResults.value = []
  isSearchMode.value = false
  isSearching.value = false
}

const onSearchKeydown = (e) => {
  if (e.key === 'Enter') triggerSearch()
  if (e.key === 'Escape') clearSearch()
}

const getFullPath = (name) => {
  return currentPath.value === '/' ? '/' + name : currentPath.value + '/' + name
}

// ── DISKS ──
const fetchDisks = async () => {
  isLoadingDisks.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/disk/info`)
    if (res.ok) disks.value = await res.json()
  } catch (e) {}
  finally { isLoadingDisks.value = false }
}

const browseMount = (mountpoint) => {
  if (mountpoint && mountpoint !== '[SWAP]') fetchFiles(mountpoint)
}

const mountDevice = async (device, label) => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/disk/mount`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ device: device.replace('/dev/', ''), label: label || device })
    })
    if (res.ok) {
      const data = await res.json()
      showToast("Success", data.message, "success")
      await fetchDisks()
      if (data.mountpoint) fetchFiles(data.mountpoint)
    } else {
      showToast("Error", await res.text(), "error")
    }
  } catch (e) {
    showToast("Error", "Mount failed", "error")
  }
}

const umountDevice = async (device) => {
  showConfirm("Unmount Device", `Unmount ${device}?`, async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/disk/umount`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ device: device.replace('/dev/', '') })
      })
      if (res.ok) {
        showToast("Success", "Device unmounted", "success")
        fetchDisks()
      } else {
        showToast("Error", await res.text(), "error")
      }
    } catch (e) {
      showToast("Error", "Umount failed", "error")
    }
  })
}

// ── UPLOAD ──
const handleUpload = async (event) => {
  if (isReadOnly.value) return showToast("Warning", "Cannot upload: read-only path", "warning")
  const fileList = event.target.files
  if (!fileList || fileList.length === 0) return
  showToast("Info", `Uploading ${fileList.length} file(s)...`)
  const fd = new FormData()
  for (let i = 0; i < fileList.length; i++) fd.append('file', fileList[i])
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/upload?path=${encodeURIComponent(currentPath.value)}`, {
      method: 'POST',
      body: fd
    })
    if (res.ok) {
      showToast("Success", "Upload complete", "success")
      fetchFiles(currentPath.value)
    } else {
      showToast("Error", await res.text(), "error")
    }
  } catch (e) {
    showToast("Error", "Upload failed", "error")
  }
}

const fetchUrl = async () => {
  if (isReadOnly.value) return showToast("Warning", "Read-only path", "warning")
  const url = prompt("Enter URL to download:")
  if (!url) return
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/fetch`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url, path: currentPath.value })
    })
    if (res.ok) {
      showToast("Success", "File fetched", "success")
      fetchFiles(currentPath.value)
    } else {
      showToast("Error", await res.text(), "error")
    }
  } catch (e) {
    showToast("Error", "Fetch failed", "error")
  }
}

// ── CONTEXT MENU ──
const ctxMenu = ref({ visible: false, x: 0, y: 0, file: null })

const openCtxMenu = (e, file) => {
  e.preventDefault()
  ctxMenu.value = { visible: true, x: e.clientX, y: e.clientY, file }
}
const closeCtxMenu = () => { ctxMenu.value.visible = false }

const ctxAction = async (action) => {
  const f = ctxMenu.value.file
  if (!f) return
  closeCtxMenu()
  // Jika file berasal dari hasil search (punya full_path), gunakan full_path langsung
  const target = f.full_path || getFullPath(f.name)

  if (action === 'open' && f.is_dir) return fetchFiles(target)
  if (action === 'download') {
    window.open(`${getActiveServerUrl()}/api/files/download?path=${encodeURIComponent(target)}`, '_blank')
    return
  }
  if (action === 'info') {
    closeCtxMenu()
    openInfoModal(target)
    return
  }
  if (action === 'edit') {
    openEditor(target)
    return
  }
  if (action === 'delete') {
    showConfirm("Delete", `Delete "${f.name}"?`, async () => {
      await fileAction({ action: 'delete', target })
    })
    return
  }
  if (action === 'rename') {
    const newName = prompt("New name:", f.name)
    if (newName && newName !== f.name) await fileAction({ action: 'rename', target, destination: newName })
    return
  }
  if (action === 'copy') {
    const dest = prompt("Copy to path:", currentPath.value + '/')
    if (dest) await fileAction({ action: 'copy', target, destination: dest + f.name })
    return
  }
  if (action === 'move') {
    const dest = prompt("Move to path:", currentPath.value + '/')
    if (dest) await fileAction({ action: 'move', target, destination: dest + f.name })
    return
  }
  if (action === 'compress') {
    await fileAction({ action: 'compress', target, destination: target + '.zip' })
    return
  }
  if (action === 'chmod') {
    closeCtxMenu()
    openChmodModal(target)
    return
  }
}

const fileAction = async (payload) => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/action`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    })
    if (res.ok) {
      showToast("Success", "Action completed", "success")
      fetchFiles(currentPath.value)
    } else {
      const err = await res.json().catch(() => ({ error: 'Unknown error' }))
      showToast("Error", err.error || err, "error")
    }
  } catch (e) {
    showToast("Error", "Action failed", "error")
  }
}

// ── TEXT EDITOR ──
const editor = ref({ visible: false, path: '', content: '', loading: false, saving: false })

// ── FILE INFO MODAL ──
const infoModal = ref({ visible: false, loading: false, data: null })

const openInfoModal = async (path) => {
  infoModal.value = { visible: true, loading: true, data: null }
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/info?path=${encodeURIComponent(path)}`)
    if (res.ok) {
      infoModal.value.data = await res.json()
    } else {
      showToast("Error", "Failed to fetch file info", "error")
      infoModal.value.visible = false
    }
  } catch (e) {
    showToast("Error", "Error fetching file info", "error")
    infoModal.value.visible = false
  } finally {
    infoModal.value.loading = false
  }
}

const formatBytes = (bytes) => {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(2)} ${units[i]} (${bytes.toLocaleString()} bytes)`
}

const formatTimestamp = (ts) => {
  if (!ts) return '-'
  return new Date(ts * 1000).toLocaleString()
}

// ── CHMOD MODAL ──
const chmodModal = ref({
  visible: false,
  path: '',
  fileName: '',
  owner: { read: false, write: false, execute: false },
  group: { read: false, write: false, execute: false },
  others: { read: false, write: false, execute: false },
  isApplying: false,
})

const chmodOctal = computed(() => {
  const c = chmodModal.value
  const calc = (p) => (p.read ? 4 : 0) + (p.write ? 2 : 0) + (p.execute ? 1 : 0)
  const result = `0${calc(c.owner)}${calc(c.group)}${calc(c.others)}`
  return result
})

const chmodSymbolic = computed(() => {
  const pStr = (p) => `${p.read ? 'r' : '-'}${p.write ? 'w' : '-'}${p.execute ? 'x' : '-'}`
  const c = chmodModal.value
  return `${pStr(c.owner)}${pStr(c.group)}${pStr(c.others)}`
})

const openChmodModal = async (path) => {
  const name = path.split('/').pop()
  // Ambil permissions saat ini dari API
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/info?path=${encodeURIComponent(path)}`)
    if (res.ok) {
      const info = await res.json()
      const oct = info.permissions_octal
      // Parse 4-digit octal string (e.g. "0644")
      const ownerDigit = parseInt(oct[1] || oct[0])
      const groupDigit = parseInt(oct[2] || '0')
      const othersDigit = parseInt(oct[3] || '0')
      const parse = (d) => ({ read: !!(d & 4), write: !!(d & 2), execute: !!(d & 1) })
      chmodModal.value = {
        visible: true,
        path,
        fileName: name,
        owner: parse(ownerDigit),
        group: parse(groupDigit),
        others: parse(othersDigit),
        isApplying: false,
      }
    }
  } catch (e) {
    showToast("Error", "Failed to read permissions", "error")
  }
}

const applyChmod = async () => {
  chmodModal.value.isApplying = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/action`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action: 'chmod', target: chmodModal.value.path, destination: chmodModal.value.chmodOctal || chmodOctal.value })
    })
    if (res.ok) {
      showToast("Success", `Permissions changed to ${chmodOctal.value}`, "success")
      chmodModal.value.visible = false
      fetchFiles(currentPath.value)
    } else {
      const err = await res.json().catch(() => ({ error: 'Failed to apply chmod' }))
      showToast("Error", err.error, "error")
    }
  } catch (e) {
    showToast("Error", "Failed to apply permissions", "error")
  } finally {
    chmodModal.value.isApplying = false
  }
}

const openEditor = async (path) => {
  editor.value = { visible: true, path, content: '', loading: true, saving: false }
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/text`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path })
    })
    const data = await res.json()
    editor.value.content = data.content || ''
  } catch (e) {
    showToast("Error", "Failed to open file", "error")
    editor.value.visible = false
  } finally {
    editor.value.loading = false
  }
}

const saveEditor = async () => {
  editor.value.saving = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/text`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: editor.value.path, content: editor.value.content })
    })
    if (res.ok) {
      showToast("Success", "File saved", "success")
      editor.value.visible = false
      fetchFiles(currentPath.value)
    } else {
      const err = await res.json().catch(() => ({ error: 'Save failed' }))
      showToast("Error", err.error, "error")
    }
  } catch (e) {
    showToast("Error", "Save failed", "error")
  } finally {
    editor.value.saving = false
  }
}

// ── FORMAT UTILS ──
const formatSize = (bytes) => {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`
}

const getFileIcon = (f) => {
  if (f.is_dir) return '📁'
  const ext = f.name.split('.').pop()?.toLowerCase()
  const icons = { txt: '📄', md: '📝', json: '🔧', yaml: '🔧', yml: '🔧', sh: '⚙️', py: '🐍', js: '🟨', ts: '🔷', vue: '💚', rs: '🦀', toml: '🔧', conf: '⚙️', log: '📋', zip: '🗜️', tar: '🗜️', gz: '🗜️', png: '🖼️', jpg: '🖼️', jpeg: '🖼️', gif: '🖼️', mp4: '🎬', mkv: '🎬', mp3: '🎵', pdf: '📕' }
  return icons[ext] || '📄'
}

// ── LIFECYCLE ──
let diskRefreshTimer = null
const searchInputRef = ref(null)

const handleGlobalKeydown = (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
    e.preventDefault()
    searchInputRef.value?.focus()
  }
}

onMounted(async () => {
  // Ambil config dulu (sequential), baru fetch files pakai homeRoot yang sudah terisi
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/config`)
    if (res.ok) {
      const config = await res.json()
      homeRoot.value = config.home_root
    }
  } catch (e) {}

  // Fetch files setelah homeRoot terisi, fetch disks 1x saja saat mount
  fetchFiles(homeRoot.value)
  fetchDisks()

  // Shortcut Ctrl+F
  window.addEventListener('keydown', handleGlobalKeydown)
})

onUnmounted(() => {
  if (diskRefreshTimer) clearInterval(diskRefreshTimer)
  window.removeEventListener('keydown', handleGlobalKeydown)
})
</script>

<template>
  <div class="flex h-[calc(100vh-200px)] gap-0 rounded-xl overflow-hidden border"
       :class="isDark ? 'border-slate-700 bg-slate-900' : 'border-slate-200 bg-white'"
       @click="closeCtxMenu">

    <!-- ── SIDEBAR: DISK MANAGER ── -->
    <div class="w-56 shrink-0 border-r flex flex-col" :class="isDark ? 'border-slate-700 bg-slate-900' : 'border-slate-200 bg-slate-50'">
      
      <!-- Sidebar Header -->
      <div class="p-3 border-b flex items-center justify-between" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
        <span class="text-xs font-bold uppercase tracking-wider text-slate-500">Storage</span>
        <button @click="fetchDisks" class="text-slate-400 hover:text-brand-500">
          <RefreshCw class="w-3.5 h-3.5" />
        </button>
      </div>

      <!-- Quick Access (Static Shortcuts) -->
      <div class="p-2 border-b" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
        <div class="text-[10px] font-bold uppercase tracking-wider text-slate-500 px-2 mb-1">Quick Access</div>
        <!-- Home -->
        <button @click="navigateHome"
          class="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-xs transition-colors text-left"
          :class="currentPath === homeRoot || currentPath.startsWith(homeRoot + '/')
            ? (isDark ? 'bg-brand-900/30 text-brand-400' : 'bg-brand-50 text-brand-700')
            : (isDark ? 'hover:bg-slate-800 text-slate-300' : 'hover:bg-slate-100 text-slate-700')">
          <Home class="w-3.5 h-3.5 shrink-0" />
          <div>
            <div class="font-semibold">Home</div>
            <div class="text-[10px] text-slate-500 font-mono truncate">{{ homeRoot }}</div>
          </div>
        </button>
        <!-- System Root -->
        <button @click="navigateTo('/')"
          class="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-xs transition-colors text-left"
          :class="currentPath === '/'
            ? (isDark ? 'bg-brand-900/30 text-brand-400' : 'bg-brand-50 text-brand-700')
            : (isDark ? 'hover:bg-slate-800 text-slate-300' : 'hover:bg-slate-100 text-slate-700')">
          <HardDrive class="w-3.5 h-3.5 shrink-0 text-slate-400" />
          <div>
            <div class="font-semibold">System Root</div>
            <div class="text-[10px] text-slate-500 font-mono">/</div>
          </div>
        </button>
      </div>

      <!-- Disk List -->
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
        <div v-if="isLoadingDisks" class="text-center py-4"><Loader2 class="w-4 h-4 animate-spin mx-auto text-brand-500" /></div>
        
        <div v-for="disk in disks" :key="disk.name">
          <!-- Disk parent info -->
          <div class="px-2 py-1">
            <div class="flex items-center gap-1.5 text-[10px] font-semibold text-slate-400 uppercase tracking-wider">
              <Usb v-if="disk.is_removable" class="w-3 h-3 text-amber-500" />
              <HardDrive v-else class="w-3 h-3 text-blue-500" />
              {{ disk.model || disk.name }}
              <span class="text-[9px] normal-case text-slate-500">{{ disk.size }}</span>
            </div>
          </div>

          <!-- Partitions / children -->
          <div v-if="disk.children">
            <div v-for="part in disk.children" :key="part.name"
                 class="ml-2 rounded-lg p-2 mb-1 cursor-pointer transition-colors"
                 :class="part.mountpoint && part.mountpoint !== '[SWAP]' && (currentPath === part.mountpoint || currentPath.startsWith(part.mountpoint === '/' ? '//' : part.mountpoint + '/'))
                    ? (isDark ? 'bg-brand-900/30 border border-brand-700' : 'bg-brand-50 border border-brand-200')
                    : (isDark ? 'hover:bg-slate-800 border border-transparent' : 'hover:bg-slate-100 border border-transparent')"
                 @click="part.mountpoint && part.mountpoint !== '[SWAP]' ? browseMount(part.mountpoint) : null">
              <div class="flex items-center justify-between">
                <div>
                  <div class="text-xs font-semibold" :class="isDark ? 'text-slate-300' : 'text-slate-700'">
                    <!-- Label yang lebih informatif berdasarkan mountpoint -->
                    {{ part.label || (part.mountpoint === '/' ? 'System (/)' : part.mountpoint === '/boot' ? 'Boot' : part.mountpoint ? part.mountpoint.split('/').pop() : part.name) }}
                  </div>
                  <div class="text-[10px] text-slate-500">
                    {{ part.mountpoint === '[SWAP]' ? 'Swap' : (part.mountpoint || 'Not mounted') }}
                  </div>
                </div>
                <div class="flex flex-col items-end gap-1">
                  <span class="text-[9px]" :class="isDark ? 'text-slate-400' : 'text-slate-500'">{{ part.size }}</span>
                  <!-- Mount/Unmount button for removable -->
                  <button v-if="disk.is_removable && !part.mounted"
                    @click.stop="mountDevice('/dev/' + part.name, part.label)"
                    class="text-[9px] px-1.5 py-0.5 rounded bg-green-100 text-green-700 hover:bg-green-200 font-bold">
                    MOUNT
                  </button>
                  <button v-else-if="disk.is_removable && part.mounted"
                    @click.stop="umountDevice('/dev/' + part.name)"
                    class="text-[9px] px-1.5 py-0.5 rounded bg-red-100 text-red-700 hover:bg-red-200 font-bold">
                    EJECT
                  </button>
                </div>
              </div>
              <!-- Usage bar -->
              <div v-if="part.used_percent !== null && part.used_percent !== undefined" class="mt-1.5">
                <div class="w-full rounded-full h-1 overflow-hidden" :class="isDark ? 'bg-slate-700' : 'bg-slate-200'">
                  <div class="h-1 rounded-full transition-all"
                       :class="part.used_percent > 90 ? 'bg-red-500' : part.used_percent > 70 ? 'bg-amber-500' : 'bg-brand-500'"
                       :style="`width: ${part.used_percent}%`"></div>
                </div>
                <div class="text-[9px] text-slate-500 mt-0.5">{{ part.used_percent }}% used</div>
              </div>
            </div>
          </div>
          <!-- Disk without children -->
          <div v-else-if="disk.mountpoint"
               class="ml-2 rounded-lg p-2 mb-1 cursor-pointer transition-colors"
               :class="currentPath.startsWith(disk.mountpoint) 
                  ? (isDark ? 'bg-brand-900/30 border border-brand-700' : 'bg-brand-50 border border-brand-200')
                  : (isDark ? 'hover:bg-slate-800 border border-transparent' : 'hover:bg-slate-100 border border-transparent')"
               @click="browseMount(disk.mountpoint)">
            <div class="text-xs font-semibold" :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ disk.label || disk.name }}</div>
            <div class="text-[10px] text-slate-500">{{ disk.mountpoint }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ── MAIN PANE: FILE BROWSER ── -->
    <div class="flex-1 flex flex-col min-w-0">

      <!-- Toolbar -->
      <div class="shrink-0 border-b" :class="isDark ? 'border-slate-700 bg-slate-900' : 'border-slate-200 bg-white'">
        <!-- Row 1: Navigation + Actions -->
        <div class="px-3 py-2 flex items-center gap-2">
        
          <!-- Nav Buttons -->
          <button @click="navigateUp" class="p-1.5 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500" title="Go Up">
            <ArrowUp class="w-4 h-4" />
          </button>
          <button @click="navigateHome" class="p-1.5 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500" title="Home">
            <Home class="w-4 h-4" />
          </button>

          <!-- Address Bar -->
          <div class="flex-1 flex items-center gap-1 min-w-0">
            <!-- Breadcrumbs on wider screens -->
            <div class="hidden lg:flex items-center gap-0.5 flex-1 min-w-0">
              <button v-for="(crumb, i) in breadcrumbs" :key="crumb.path" @click="navigateTo(crumb.path)"
                class="flex items-center gap-0.5 text-xs hover:text-brand-600 dark:hover:text-brand-400 shrink-0"
                :class="i === breadcrumbs.length - 1 ? (isDark ? 'text-slate-100 font-bold' : 'text-slate-800 font-bold') : (isDark ? 'text-slate-400' : 'text-slate-500')">
                {{ crumb.label }}
                <ChevronRight v-if="i < breadcrumbs.length - 1" class="w-3 h-3 opacity-50" />
              </button>
            </div>
            <!-- Input bar on small screens -->
            <input v-model="pathInput" @keyup.enter="onPathInputEnter" type="text"
              class="flex-1 min-w-0 text-xs px-2 py-1 rounded border font-mono"
              :class="isDark ? 'bg-slate-800 border-slate-600 text-slate-200' : 'bg-white border-slate-300 text-slate-700'"
              placeholder="Path..." />
          </div>

          <!-- Read-Only badge -->
          <div v-if="isReadOnly" class="flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400 shrink-0">
            <Lock class="w-3 h-3" /> Read-Only
          </div>

          <!-- Action Buttons (hidden in read-only) -->
          <template v-if="!isReadOnly">
            <label class="cursor-pointer p-1.5 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500" title="Upload">
              <Upload class="w-4 h-4" />
              <input type="file" multiple class="hidden" @change="handleUpload" />
            </label>
            <button @click="fetchUrl" class="p-1.5 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500" title="Fetch URL">
              <DownloadCloud class="w-4 h-4" />
            </button>
          </template>

          <button @click="fetchFiles(currentPath)" class="p-1.5 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500" title="Refresh">
            <RefreshCw class="w-4 h-4" :class="isLoadingFiles ? 'animate-spin' : ''" />
          </button>

          <!-- Separator -->
          <div class="w-px h-5 bg-slate-200 dark:bg-slate-700 shrink-0"></div>

          <!-- View Mode Toggle -->
          <div class="flex items-center gap-0.5 shrink-0">
            <button @click="viewMode = 'list'" class="p-1.5 rounded transition-colors" :class="viewMode === 'list' ? 'bg-brand-100 text-brand-600 dark:bg-brand-900/30 dark:text-brand-400' : 'text-slate-400 hover:text-slate-600 hover:bg-slate-100 dark:hover:bg-slate-700'" title="List View">
              <List class="w-4 h-4" />
            </button>
            <button @click="viewMode = 'grid'" class="p-1.5 rounded transition-colors" :class="viewMode === 'grid' ? 'bg-brand-100 text-brand-600 dark:bg-brand-900/30 dark:text-brand-400' : 'text-slate-400 hover:text-slate-600 hover:bg-slate-100 dark:hover:bg-slate-700'" title="Grid View">
              <LayoutGrid class="w-4 h-4" />
            </button>
            <button @click="viewMode = 'compact'" class="p-1.5 rounded transition-colors" :class="viewMode === 'compact' ? 'bg-brand-100 text-brand-600 dark:bg-brand-900/30 dark:text-brand-400' : 'text-slate-400 hover:text-slate-600 hover:bg-slate-100 dark:hover:bg-slate-700'" title="Compact View">
              <AlignJustify class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- Row 2: Search Bar -->
        <div class="px-3 pb-2 flex items-center gap-2">
          <div class="flex-1 flex items-center gap-2 border rounded-lg px-3 py-1.5 transition-colors"
               :class="isSearchMode ? (isDark ? 'border-brand-500 bg-brand-900/10' : 'border-brand-400 bg-brand-50') : (isDark ? 'border-slate-700 bg-slate-800' : 'border-slate-200 bg-white')">
            <Search class="w-3.5 h-3.5 shrink-0" :class="isSearchMode ? 'text-brand-500' : 'text-slate-400'" />
            <input ref="searchInputRef" v-model="searchQuery" @keydown="onSearchKeydown" type="text"
              class="flex-1 text-xs bg-transparent focus:outline-none"
              :class="isDark ? 'text-slate-200 placeholder-slate-500' : 'text-slate-700 placeholder-slate-400'"
              placeholder="Search files... (Enter to search recursively, Esc to clear, Ctrl+F to focus)" />
            <Loader2 v-if="isSearching" class="w-3.5 h-3.5 animate-spin text-brand-500 shrink-0" />
            <button v-else-if="searchQuery" @click="clearSearch" class="text-slate-400 hover:text-slate-600 shrink-0">
              <X class="w-3.5 h-3.5" />
            </button>
          </div>
          <button @click="triggerSearch" class="btn-primary py-1 px-3 text-xs whitespace-nowrap" :disabled="isSearching || searchQuery.length < 2">
            <Search class="w-3 h-3" /> Search
          </button>
        </div>

        <!-- Search Mode Indicator -->
        <div v-if="isSearchMode" class="px-3 pb-2 flex items-center justify-between text-[10px]"
             :class="isDark ? 'text-slate-400' : 'text-slate-500'">
          <span>
            <span class="font-bold text-brand-500">{{ searchResults.length }}</span> result(s) for
            "<span class="font-mono">{{ searchQuery }}</span>" in
            <span class="font-mono text-[10px]">{{ currentPath }}</span>
          </span>
          <button @click="clearSearch" class="text-brand-500 hover:underline font-semibold">Clear Search</button>
        </div>
      </div>

      <!-- File List -->
      <div class="flex-1 overflow-y-auto">
        <div v-if="isLoadingFiles || isSearching" class="flex items-center justify-center h-32">
          <Loader2 class="w-6 h-6 animate-spin text-brand-500" />
        </div>

        <!-- ── LIST VIEW ── -->
        <table v-else-if="viewMode === 'list'" class="w-full">
          <thead class="sticky top-0 text-xs border-b" :class="isDark ? 'bg-slate-900 border-slate-700 text-slate-400' : 'bg-white border-slate-100 text-slate-500'">
            <tr>
              <th class="text-left px-4 py-2 font-semibold">Name</th>
              <th v-if="isSearchMode" class="text-left px-4 py-2 font-semibold">Path</th>
              <th class="text-right px-4 py-2 font-semibold w-20">Size</th>
              <th class="text-right px-4 py-2 font-semibold w-24">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y" :class="isDark ? 'divide-slate-800' : 'divide-slate-50'">
            <!-- Parent dir shortcut (hidden in search mode) -->
            <tr v-if="currentPath !== '/' && !isSearchMode" @dblclick="navigateUp"
              class="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors">
              <td class="px-4 py-2 text-sm flex items-center gap-2 text-slate-400">
                <span>📁</span> ..
              </td>
              <td></td><td></td>
            </tr>

            <tr v-for="f in displayFiles" :key="f.full_path || f.name"
              class="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors group"
              @dblclick="f.is_dir ? navigateTo(f.full_path || getFullPath(f.name)) : openEditor(f.full_path || getFullPath(f.name))"
              @contextmenu.prevent="openCtxMenu($event, f)">
              <td class="px-4 py-1.5">
                <div class="flex items-center gap-2">
                  <span class="text-base leading-none">{{ getFileIcon(f) }}</span>
                  <div>
                    <div class="text-sm" :class="f.is_dir ? (isDark ? 'text-brand-400 font-medium' : 'text-brand-700 font-medium') : (isDark ? 'text-slate-200' : 'text-slate-700')">
                      {{ f.name }}
                    </div>
                  </div>
                  <Lock v-if="!f.writable" class="w-3 h-3 text-slate-400 opacity-60" />
                </div>
              </td>
              <td v-if="isSearchMode" class="px-4 py-1.5 text-xs text-slate-500 font-mono truncate max-w-[200px]" :title="f.relative_path">
                {{ f.relative_path }}
              </td>
              <td class="px-4 py-1.5 text-right text-xs text-slate-500">
                {{ f.is_dir ? '—' : formatSize(f.size) }}
              </td>
              <td class="px-4 py-1.5 text-right">
                <div class="flex items-center justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                  <a :href="`${getActiveServerUrl()}/api/files/download?path=${encodeURIComponent(f.full_path || getFullPath(f.name))}`" target="_blank" download
                    v-if="!f.is_dir"
                    class="p-1 rounded hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-400 hover:text-brand-600">
                    <Download class="w-3.5 h-3.5" />
                  </a>
                  <button v-if="!f.is_dir && f.writable" @click.stop="openEditor(f.full_path || getFullPath(f.name))"
                    class="p-1 rounded hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-400 hover:text-brand-600">
                    <FileEdit class="w-3.5 h-3.5" />
                  </button>
                  <button v-if="f.writable" @click.stop="showConfirm('Delete', `Delete '${f.name}'?`, () => fileAction({ action: 'delete', target: f.full_path || getFullPath(f.name) }))"
                    class="p-1 rounded hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-400 hover:text-red-500">
                    <Trash2 class="w-3.5 h-3.5" />
                  </button>
                </div>
              </td>
            </tr>

            <tr v-if="displayFiles.length === 0 && !isSearchMode">
              <td :colspan="isSearchMode ? 4 : 3" class="text-center py-12 text-slate-400 text-sm">
                <FolderTree class="w-8 h-8 mx-auto mb-2 opacity-30" />
                Empty directory
              </td>
            </tr>
            <tr v-if="displayFiles.length === 0 && isSearchMode">
              <td :colspan="isSearchMode ? 4 : 3" class="text-center py-12 text-slate-400 text-sm">
                <Search class="w-8 h-8 mx-auto mb-2 opacity-30" />
                No files found matching "{{ searchQuery }}"
              </td>
            </tr>
          </tbody>
        </table>

        <!-- ── GRID VIEW ── -->
        <div v-else-if="viewMode === 'grid'" class="p-4">
          <!-- Parent dir -->
          <div v-if="currentPath !== '/' && !isSearchMode" @dblclick="navigateUp"
            class="inline-flex flex-col items-center p-3 rounded-xl border cursor-pointer transition-colors w-28 mb-3 mr-2"
            :class="isDark ? 'border-slate-700 hover:bg-slate-800' : 'border-slate-200 hover:bg-slate-50'">
            <span class="text-4xl">📁</span>
            <div class="text-[10px] text-slate-400 mt-2 font-mono">..</div>
          </div>

          <div class="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-8 gap-3">
            <div v-for="f in displayFiles" :key="f.full_path || f.name"
              class="group flex flex-col items-center p-3 rounded-xl border cursor-pointer transition-colors select-none"
              :class="isDark ? 'border-slate-700 hover:bg-slate-800 hover:border-slate-600' : 'border-slate-200 hover:bg-brand-50 hover:border-brand-200'"
              @dblclick="f.is_dir ? navigateTo(f.full_path || getFullPath(f.name)) : openEditor(f.full_path || getFullPath(f.name))"
              @contextmenu.prevent="openCtxMenu($event, f)">
              <!-- Image Thumbnail atau Emoji Icon -->
              <div class="w-16 h-16 flex items-center justify-center rounded-lg overflow-hidden shrink-0"
                   :class="isDark ? 'bg-slate-700/50' : 'bg-slate-100'">
                <img v-if="!f.is_dir && isImage(f)"
                  :src="thumbnailUrl(f.full_path || getFullPath(f.name), !!f.full_path)"
                  :alt="f.name"
                  class="w-full h-full object-cover"
                  loading="lazy"
                  @error="$event.target.style.display='none'; $event.target.nextSibling.style.display='block'"
                />
                <span class="text-3xl leading-none" :style="!f.is_dir && isImage(f) ? 'display:none' : ''">{{ getFileIcon(f) }}</span>
              </div>
              <!-- Name + Size -->
              <div class="mt-2 text-center w-full">
                <div class="text-[11px] font-medium break-words line-clamp-2 leading-tight" :class="f.is_dir ? (isDark ? 'text-brand-400' : 'text-brand-700') : (isDark ? 'text-slate-200' : 'text-slate-700')">
                  {{ f.name }}
                </div>
                <div class="text-[9px] text-slate-400 mt-0.5">{{ f.is_dir ? 'Folder' : formatSize(f.size) }}</div>
                <div v-if="isSearchMode" class="text-[9px] text-slate-500 font-mono truncate mt-0.5" :title="f.relative_path">
                  {{ f.relative_path }}
                </div>
              </div>
              <!-- Lock icon for read-only -->
              <Lock v-if="!f.writable" class="w-3 h-3 text-slate-400 opacity-60 mt-1" />
            </div>
          </div>

          <div v-if="displayFiles.length === 0" class="text-center py-12 text-slate-400 text-sm">
            <FolderTree class="w-8 h-8 mx-auto mb-2 opacity-30" />
            {{ isSearchMode ? `No files found matching "${searchQuery}"` : 'Empty directory' }}
          </div>
        </div>

        <!-- ── COMPACT VIEW ── -->
        <div v-else-if="viewMode === 'compact'" class="py-1">
          <!-- Parent dir -->
          <div v-if="currentPath !== '/' && !isSearchMode" @dblclick="navigateUp"
            class="flex items-center gap-2 px-3 py-0.5 cursor-pointer text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800/50 text-xs">
            <span class="text-sm">📁</span> ..
          </div>

          <div v-for="f in displayFiles" :key="f.full_path || f.name"
            class="group flex items-center gap-2 px-3 py-0.5 cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/30 transition-colors"
            @dblclick="f.is_dir ? navigateTo(f.full_path || getFullPath(f.name)) : openEditor(f.full_path || getFullPath(f.name))"
            @contextmenu.prevent="openCtxMenu($event, f)">
            <span class="text-sm leading-none shrink-0">{{ getFileIcon(f) }}</span>
            <div class="flex-1 min-w-0">
              <span class="text-xs" :class="f.is_dir ? (isDark ? 'text-brand-400' : 'text-brand-700') : (isDark ? 'text-slate-200' : 'text-slate-700')">
                {{ f.name }}
              </span>
              <span v-if="isSearchMode" class="text-[10px] text-slate-500 font-mono ml-2 opacity-70">{{ f.relative_path }}</span>
            </div>
            <span class="text-[10px] text-slate-400 shrink-0">{{ f.is_dir ? '' : formatSize(f.size) }}</span>
            <Lock v-if="!f.writable" class="w-3 h-3 text-slate-400 opacity-40 shrink-0" />
            <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
              <a :href="`${getActiveServerUrl()}/api/files/download?path=${encodeURIComponent(f.full_path || getFullPath(f.name))}`" target="_blank" download
                v-if="!f.is_dir"
                class="p-1 rounded hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-400">
                <Download class="w-3 h-3" />
              </a>
              <button v-if="f.writable" @click.stop="showConfirm('Delete', `Delete '${f.name}'?`, () => fileAction({ action: 'delete', target: f.full_path || getFullPath(f.name) }))"
                class="p-1 rounded hover:bg-red-100 dark:hover:bg-red-900/20 text-slate-400 hover:text-red-500">
                <Trash2 class="w-3 h-3" />
              </button>
            </div>
          </div>

          <div v-if="displayFiles.length === 0" class="text-center py-8 text-slate-400 text-xs">
            {{ isSearchMode ? `No files found matching "${searchQuery}"` : 'Empty directory' }}
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- ── CONTEXT MENU ── -->
  <Teleport to="body">
    <div v-if="ctxMenu.visible" class="fixed z-[200] rounded-xl shadow-xl border py-1 min-w-[160px] text-sm"
         :class="isDark ? 'bg-slate-800 border-slate-700' : 'bg-white border-slate-200'"
         :style="`top: ${ctxMenu.y}px; left: ${ctxMenu.x}px`"
         @click.stop>
      <button @click="ctxAction('open')" v-if="ctxMenu.file?.is_dir" class="w-full px-4 py-2 text-left hover:bg-slate-100 dark:hover:bg-slate-700 flex items-center gap-2">
        <Folder class="w-4 h-4 text-brand-500" /> Open
      </button>
      <button @click="ctxAction('download')" v-if="!ctxMenu.file?.is_dir" class="w-full px-4 py-2 text-left hover:bg-slate-100 dark:hover:bg-slate-700 flex items-center gap-2">
        <Download class="w-4 h-4 text-green-500" /> Download
      </button>
      <button @click="ctxAction('info')" class="w-full px-4 py-2 text-left hover:bg-slate-100 dark:hover:bg-slate-700 flex items-center gap-2">
        <FolderTree class="w-4 h-4 text-slate-400" /> Info
      </button>
      <template v-if="ctxMenu.file?.writable">
        <hr class="my-1" :class="isDark ? 'border-slate-700' : 'border-slate-100'" />
        <button @click="ctxAction('edit')" v-if="!ctxMenu.file?.is_dir" class="w-full px-4 py-2 text-left hover:bg-slate-100 dark:hover:bg-slate-700 flex items-center gap-2">
          <FileEdit class="w-4 h-4 text-blue-400" /> Edit Text
        </button>
        <button @click="ctxAction('rename')" class="w-full px-4 py-2 text-left hover:bg-slate-100 dark:hover:bg-slate-700 flex items-center gap-2">
          <FileText class="w-4 h-4 text-amber-400" /> Rename
        </button>
        <button @click="ctxAction('copy')" class="w-full px-4 py-2 text-left hover:bg-slate-100 dark:hover:bg-slate-700 flex items-center gap-2">
          <Copy class="w-4 h-4 text-blue-400" /> Copy
        </button>
        <button @click="ctxAction('move')" class="w-full px-4 py-2 text-left hover:bg-slate-100 dark:hover:bg-slate-700 flex items-center gap-2">
          <Move class="w-4 h-4 text-purple-400" /> Move
        </button>
        <button @click="ctxAction('compress')" v-if="ctxMenu.file?.is_dir" class="w-full px-4 py-2 text-left hover:bg-slate-100 dark:hover:bg-slate-700 flex items-center gap-2">
          <Archive class="w-4 h-4 text-orange-400" /> Compress to ZIP
        </button>
        <button @click="ctxAction('chmod')" class="w-full px-4 py-2 text-left hover:bg-slate-100 dark:hover:bg-slate-700 flex items-center gap-2">
          <Info class="w-4 h-4 text-slate-400" /> Permissions (chmod)
        </button>
        <hr class="my-1" :class="isDark ? 'border-slate-700' : 'border-slate-100'" />
        <button @click="ctxAction('delete')" class="w-full px-4 py-2 text-left text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 flex items-center gap-2">
          <Trash2 class="w-4 h-4" /> Delete
        </button>
      </template>
      <div v-else class="px-4 py-2 text-xs text-slate-400 flex items-center gap-1.5">
        <Lock class="w-3 h-3" /> Read-only path
      </div>
    </div>
  </Teleport>

  <!-- ── TEXT EDITOR MODAL ── -->
  <Teleport to="body">
    <div v-if="editor.visible" class="fixed inset-0 z-[100] backdrop-blur-sm flex items-center justify-center p-4"
         :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
      <div class="rounded-xl shadow-2xl w-full max-w-4xl h-[80vh] flex flex-col overflow-hidden"
           :class="isDark ? 'bg-slate-800' : 'bg-white'">
        <div class="p-3 border-b flex items-center justify-between shrink-0"
             :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-200'">
          <div>
            <h3 class="font-bold text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-800'">
              <FileEdit class="w-4 h-4 inline mr-1.5 text-brand-500" />{{ editor.path.split('/').pop() }}
            </h3>
            <p class="text-[10px] font-mono text-slate-500 mt-0.5">{{ editor.path }}</p>
          </div>
          <div class="flex gap-2">
            <button @click="saveEditor" class="btn-primary py-1.5 px-3 text-xs" :disabled="editor.saving">
              <Save class="w-3.5 h-3.5" /> {{ editor.saving ? 'Saving...' : 'Save' }}
            </button>
            <button @click="editor.visible = false" class="p-1.5 rounded text-slate-400 hover:text-slate-200">
              <X class="w-4 h-4" />
            </button>
          </div>
        </div>
        <div v-if="editor.loading" class="flex-1 flex items-center justify-center">
          <Loader2 class="w-6 h-6 animate-spin text-brand-500" />
        </div>
        <textarea v-else v-model="editor.content"
          class="flex-1 w-full font-mono text-xs p-4 resize-none focus:outline-none"
          :class="isDark ? 'bg-slate-900 text-slate-300' : 'bg-white text-slate-800'"></textarea>
      </div>
    </div>
  </Teleport>

  <!-- ── FILE INFO MODAL ── -->
  <Teleport to="body">
    <div v-if="infoModal.visible" class="fixed inset-0 z-[200] backdrop-blur-sm flex items-center justify-center p-4"
         :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/60'">
      <div class="rounded-xl shadow-2xl w-full max-w-md overflow-hidden" :class="isDark ? 'bg-slate-800' : 'bg-white'">
        <div class="p-4 border-b flex items-center justify-between" :class="isDark ? 'border-slate-700 bg-slate-900' : 'border-slate-200 bg-slate-50'">
          <h3 class="font-bold text-sm flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'">
            <Info class="w-4 h-4 text-brand-500" /> File Information
          </h3>
          <button @click="infoModal.visible = false" class="text-slate-400 hover:text-slate-200"><X class="w-4 h-4" /></button>
        </div>
        <div class="p-4">
          <div v-if="infoModal.loading" class="flex justify-center py-8"><Loader2 class="w-6 h-6 animate-spin text-brand-500" /></div>
          <dl v-else-if="infoModal.data" class="space-y-3">
            <!-- Name -->
            <div class="flex gap-3">
              <dt class="w-28 shrink-0 text-xs font-semibold text-slate-500 pt-0.5">Name</dt>
              <dd class="flex-1 text-sm font-mono break-all" :class="isDark ? 'text-slate-200' : 'text-slate-800'">{{ infoModal.data.name }}</dd>
            </div>
            <!-- Type -->
            <div class="flex gap-3">
              <dt class="w-28 shrink-0 text-xs font-semibold text-slate-500 pt-0.5">Type</dt>
              <dd class="text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-800'">
                <span class="px-2 py-0.5 rounded text-[10px] font-bold" :class="infoModal.data.is_dir ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400' : 'bg-slate-100 text-slate-600 dark:bg-slate-700 dark:text-slate-300'">
                  {{ infoModal.data.is_dir ? 'Directory' : 'File' }}
                </span>
              </dd>
            </div>
            <!-- Path -->
            <div class="flex gap-3">
              <dt class="w-28 shrink-0 text-xs font-semibold text-slate-500 pt-0.5">Full Path</dt>
              <dd class="flex-1 text-xs font-mono break-all" :class="isDark ? 'text-slate-300' : 'text-slate-600'">{{ infoModal.data.path }}</dd>
            </div>
            <!-- Size -->
            <div class="flex gap-3">
              <dt class="w-28 shrink-0 text-xs font-semibold text-slate-500 pt-0.5">Size</dt>
              <dd class="text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-800'">{{ infoModal.data.is_dir ? '—' : formatBytes(infoModal.data.size_bytes) }}</dd>
            </div>
            <!-- Owner -->
            <div class="flex gap-3">
              <dt class="w-28 shrink-0 text-xs font-semibold text-slate-500 pt-0.5">Owner</dt>
              <dd class="text-sm font-mono" :class="isDark ? 'text-slate-200' : 'text-slate-800'">{{ infoModal.data.owner }}</dd>
            </div>
            <!-- Permissions -->
            <div class="flex gap-3">
              <dt class="w-28 shrink-0 text-xs font-semibold text-slate-500 pt-0.5">Permissions</dt>
              <dd>
                <div class="flex items-center gap-2">
                  <code class="text-xs font-mono px-2 py-0.5 rounded" :class="isDark ? 'bg-slate-700 text-green-400' : 'bg-slate-100 text-slate-800'">{{ infoModal.data.permissions_octal }}</code>
                  <code class="text-xs font-mono px-2 py-0.5 rounded" :class="isDark ? 'bg-slate-700 text-green-400' : 'bg-slate-100 text-slate-800'">{{ infoModal.data.permissions_symbolic }}</code>
                </div>
              </dd>
            </div>
            <!-- Modified -->
            <div class="flex gap-3">
              <dt class="w-28 shrink-0 text-xs font-semibold text-slate-500 pt-0.5">Modified</dt>
              <dd class="text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-800'">{{ formatTimestamp(infoModal.data.modified_at) }}</dd>
            </div>
          </dl>
        </div>
      </div>
    </div>
  </Teleport>

  <!-- ── CHMOD MODAL ── -->
  <Teleport to="body">
    <div v-if="chmodModal.visible" class="fixed inset-0 z-[200] backdrop-blur-sm flex items-center justify-center p-4"
         :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/60'">
      <div class="rounded-xl shadow-2xl w-full max-w-sm overflow-hidden" :class="isDark ? 'bg-slate-800' : 'bg-white'">
        <div class="p-4 border-b flex items-center justify-between" :class="isDark ? 'border-slate-700 bg-slate-900' : 'border-slate-200 bg-slate-50'">
          <h3 class="font-bold text-sm flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'">
            <Lock class="w-4 h-4 text-brand-500" /> Permissions
          </h3>
          <button @click="chmodModal.visible = false" class="text-slate-400 hover:text-slate-200"><X class="w-4 h-4" /></button>
        </div>
        <div class="p-4 space-y-4">
          <p class="text-xs text-slate-500 font-mono truncate" :title="chmodModal.path">{{ chmodModal.fileName }}</p>

          <!-- Permission Grid -->
          <table class="w-full text-sm">
            <thead>
              <tr>
                <th class="text-left text-xs text-slate-500 pb-2 font-semibold w-20"></th>
                <th class="text-center text-xs text-slate-500 pb-2 font-semibold">Read</th>
                <th class="text-center text-xs text-slate-500 pb-2 font-semibold">Write</th>
                <th class="text-center text-xs text-slate-500 pb-2 font-semibold">Execute</th>
                <th class="text-right text-xs text-slate-500 pb-2 font-semibold pr-1">Value</th>
              </tr>
            </thead>
            <tbody class="divide-y" :class="isDark ? 'divide-slate-700' : 'divide-slate-100'">
              <!-- Owner -->
              <tr v-for="(perms, label) in [{ label: 'Owner', key: 'owner' }, { label: 'Group', key: 'group' }, { label: 'Others', key: 'others' }]" :key="label">
                <td class="py-2.5 text-xs font-semibold" :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ perms.label }}</td>
                <td class="py-2.5 text-center">
                  <input type="checkbox" v-model="chmodModal[perms.key].read" class="w-4 h-4 rounded text-brand-600 cursor-pointer" />
                </td>
                <td class="py-2.5 text-center">
                  <input type="checkbox" v-model="chmodModal[perms.key].write" class="w-4 h-4 rounded text-brand-600 cursor-pointer" />
                </td>
                <td class="py-2.5 text-center">
                  <input type="checkbox" v-model="chmodModal[perms.key].execute" class="w-4 h-4 rounded text-brand-600 cursor-pointer" />
                </td>
                <td class="py-2.5 text-right font-mono text-xs pr-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
                  {{ (chmodModal[perms.key].read ? 4 : 0) + (chmodModal[perms.key].write ? 2 : 0) + (chmodModal[perms.key].execute ? 1 : 0) }}
                </td>
              </tr>
            </tbody>
          </table>

          <!-- Preview -->
          <div class="flex items-center gap-3 px-3 py-2 rounded-lg" :class="isDark ? 'bg-slate-900' : 'bg-slate-50'">
            <span class="text-xs text-slate-500">Preview:</span>
            <code class="font-mono text-sm font-bold" :class="isDark ? 'text-green-400' : 'text-slate-800'">{{ chmodOctal }}</code>
            <code class="font-mono text-xs" :class="isDark ? 'text-slate-400' : 'text-slate-500'">({{ chmodSymbolic }})</code>
          </div>

          <!-- Actions -->
          <div class="flex justify-end gap-2 pt-1">
            <button @click="chmodModal.visible = false" class="btn-outline text-xs">Cancel</button>
            <button @click="applyChmod" class="btn-primary text-xs" :disabled="chmodModal.isApplying">
              <Loader2 v-if="chmodModal.isApplying" class="w-3.5 h-3.5 animate-spin" />
              {{ chmodModal.isApplying ? 'Applying...' : 'Apply' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>

</template>