<script setup>
import { ref, onMounted, computed } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import {
  FolderTree, DownloadCloud, Upload, ArrowUp, Folder, FileText, Download,
  X, Save, Trash2, Archive, FileEdit, Move, Copy, Info, HardDrive, Usb,
  RefreshCw, Lock, ChevronRight, Home, AlertTriangle, Loader2
} from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
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
  const target = getFullPath(f.name)

  if (action === 'open' && f.is_dir) return fetchFiles(target)
  if (action === 'download') {
    window.open(`${getActiveServerUrl()}/api/files/download?path=${encodeURIComponent(target)}`, '_blank')
    return
  }
  if (action === 'info') {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/files/info?path=${encodeURIComponent(target)}`)
      if (res.ok) {
        const info = await res.json()
        showToast("Info", `${info.name} — ${(info.size_bytes / 1024).toFixed(1)} KB — ${info.permissions_octal}`, "info")
      }
    } catch (e) {}
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
    const perm = prompt("Permissions (e.g. 0755):", "0644")
    if (perm) await fileAction({ action: 'chmod', target, destination: perm })
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
onMounted(async () => {
  // Ambil config (home_root)
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/config`)
    if (res.ok) {
      const config = await res.json()
      homeRoot.value = config.home_root
    }
  } catch (e) {}

  await Promise.all([fetchFiles(homeRoot.value), fetchDisks()])
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
                 :class="part.mountpoint && currentPath.startsWith(part.mountpoint) 
                    ? (isDark ? 'bg-brand-900/30 border border-brand-700' : 'bg-brand-50 border border-brand-200')
                    : (isDark ? 'hover:bg-slate-800 border border-transparent' : 'hover:bg-slate-100 border border-transparent')"
                 @click="part.mounted && part.mountpoint ? browseMount(part.mountpoint) : null">
              <div class="flex items-center justify-between">
                <div>
                  <div class="text-xs font-semibold" :class="isDark ? 'text-slate-300' : 'text-slate-700'">
                    {{ part.label || part.name }}
                  </div>
                  <div class="text-[10px] text-slate-500">{{ part.mountpoint || 'Not mounted' }}</div>
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
      <div class="shrink-0 border-b px-3 py-2 flex items-center gap-2" :class="isDark ? 'border-slate-700 bg-slate-900' : 'border-slate-200 bg-white'">
        
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
      </div>

      <!-- File List -->
      <div class="flex-1 overflow-y-auto">
        <div v-if="isLoadingFiles" class="flex items-center justify-center h-32">
          <Loader2 class="w-6 h-6 animate-spin text-brand-500" />
        </div>

        <table v-else class="w-full">
          <thead class="sticky top-0 text-xs border-b" :class="isDark ? 'bg-slate-900 border-slate-700 text-slate-400' : 'bg-white border-slate-100 text-slate-500'">
            <tr>
              <th class="text-left px-4 py-2 font-semibold">Name</th>
              <th class="text-right px-4 py-2 font-semibold w-24">Size</th>
              <th class="text-right px-4 py-2 font-semibold w-24">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y" :class="isDark ? 'divide-slate-800' : 'divide-slate-50'">
            <!-- Parent dir shortcut -->
            <tr v-if="currentPath !== '/'" @dblclick="navigateUp"
              class="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors">
              <td class="px-4 py-2 text-sm flex items-center gap-2 text-slate-400">
                <span>📁</span> ..
              </td>
              <td></td><td></td>
            </tr>

            <tr v-for="f in files" :key="f.name"
              class="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors group"
              @dblclick="f.is_dir ? navigateTo(getFullPath(f.name)) : openEditor(getFullPath(f.name))"
              @contextmenu.prevent="openCtxMenu($event, f)">
              <td class="px-4 py-1.5">
                <div class="flex items-center gap-2">
                  <span class="text-base leading-none">{{ getFileIcon(f) }}</span>
                  <span class="text-sm" :class="f.is_dir ? (isDark ? 'text-brand-400 font-medium' : 'text-brand-700 font-medium') : (isDark ? 'text-slate-200' : 'text-slate-700')">
                    {{ f.name }}
                  </span>
                  <!-- read-only per file badge -->
                  <Lock v-if="!f.writable" class="w-3 h-3 text-slate-400 opacity-60" />
                </div>
              </td>
              <td class="px-4 py-1.5 text-right text-xs text-slate-500">
                {{ f.is_dir ? '—' : formatSize(f.size) }}
              </td>
              <td class="px-4 py-1.5 text-right">
                <div class="flex items-center justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                  <a v-if="!f.is_dir" :href="`${getActiveServerUrl()}/api/files/download?path=${encodeURIComponent(getFullPath(f.name))}`" target="_blank" download
                    class="p-1 rounded hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-400 hover:text-brand-600">
                    <Download class="w-3.5 h-3.5" />
                  </a>
                  <button v-if="!f.is_dir && f.writable" @click.stop="openEditor(getFullPath(f.name))"
                    class="p-1 rounded hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-400 hover:text-brand-600">
                    <FileEdit class="w-3.5 h-3.5" />
                  </button>
                  <button v-if="f.writable" @click.stop="showConfirm('Delete', `Delete '${f.name}'?`, () => fileAction({ action: 'delete', target: getFullPath(f.name) }))"
                    class="p-1 rounded hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-400 hover:text-red-500">
                    <Trash2 class="w-3.5 h-3.5" />
                  </button>
                </div>
              </td>
            </tr>

            <tr v-if="files.length === 0">
              <td colspan="3" class="text-center py-12 text-slate-400 text-sm">
                <FolderTree class="w-8 h-8 mx-auto mb-2 opacity-30" />
                Empty directory
              </td>
            </tr>
          </tbody>
        </table>
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
</template>
