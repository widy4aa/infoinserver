<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import { FolderTree, DownloadCloud, Upload, ArrowUp, Folder, FileText, Download, MoreVertical, X, Save, Edit3, Trash2, Scissors, Archive, FileEdit, Move, Copy, Info } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast, showConfirm } = useToastStore()
const { isDark } = useThemeStore()

const files = ref([])
const currentPath = ref('/')
const msg = ref('')
const isError = ref(false)

const fetchFiles = async (path) => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/list?path=${encodeURIComponent(path)}`)
    if(!res.ok) throw new Error()
    files.value = await res.json()
    currentPath.value = path
  } catch (e) {
    if(path !== '/') setTimeout(() => fetchFiles('/'), 1000)
  }
}

const navigateUp = () => {
  if (currentPath.value === '/') return
  const parts = currentPath.value.split('/').filter(p => p !== '')
  parts.pop()
  fetchFiles('/' + parts.join('/'))
}

// ── UPLOAD & FETCH URL ──
const handleUpload = async (event) => {
  const fileList = event.target.files
  if(!fileList || fileList.length === 0) return
  
  showToast("Info", `Uploading ${fileList.length} file(s)...`)
  
  const fd = new FormData()
  for(let i=0; i<fileList.length; i++) fd.append('file', fileList[i])
    
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/upload?path=${encodeURIComponent(currentPath.value)}`, {
      method: 'POST',
      body: fd
    })
    const data = await res.json()
    if(res.ok) {
      showToast("Success", data.message, "success")
      fetchFiles(currentPath.value)
    } else throw new Error(data.error || data)
  } catch (e) {
    showToast("Error", e.message, "error")
  } finally {
    event.target.value = ''
  }
}

const promptFetch = async () => {
  const url = prompt("Masukkan URL file untuk di-fetch (wget):")
  if(!url) return
  showToast("Info", "Fetching from URL...")
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/fetch`, {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({ url, path: currentPath.value })
    })
    const data = await res.json()
    if(res.ok) {
      showToast("Success", data.message, "success")
      fetchFiles(currentPath.value)
    } else throw new Error(data.error || data)
  } catch(e) {
    showToast("Error", e.message, "error")
  }
}

const formatSize = (bytes) => {
  if (bytes === 0) return '0 B'
  const k = 1024, dm = 2, sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i]
}

// ── CONTEXT MENU (Klik Kanan) ──
const contextMenu = ref({ visible: false, x: 0, y: 0, file: null })
const hideContextMenu = () => { contextMenu.value.visible = false }

const showContextMenu = (event, file) => {
  event.preventDefault()
  contextMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    file
  }
}

// ── MODALS STATE ──
const modals = ref({
  rename: { show: false, newName: '' },
  move: { show: false, destPath: '' },
  copy: { show: false, destPath: '' },
  compress: { show: false, zipName: '' },
  extract: { show: false, destPath: '', password: '' },
  info: { show: false, data: null, newPerms: '', isLoading: false },
  editor: { show: false, content: '', originalContent: '', isLoading: false }
})

const getFullFilePath = (fileName) => {
  return currentPath.value === '/' ? `/${fileName}` : `${currentPath.value}/${fileName}`
}

// Helper Action API
const doAction = async (action, targetPath, destPath = null, password = null) => {
  showToast("Info", `Processing ${action}...`)
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/action`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action, target: targetPath, destination: destPath, password })
    })
    const data = await res.json()
    if (res.ok) {
      showToast("Success", data.message || `${action} complete`, "success")
      fetchFiles(currentPath.value)
      return true
    } else throw new Error(data.error || "Failed")
  } catch (e) {
    showToast("Error", e.message, "error")
    return false
  }
}

// ── ACTIONS ──
const actionRename = () => {
  modals.value.rename.newName = contextMenu.value.file.name
  modals.value.rename.show = true
  hideContextMenu()
}
const submitRename = async () => {
  const target = getFullFilePath(contextMenu.value.file.name)
  if (await doAction('rename', target, modals.value.rename.newName)) {
    modals.value.rename.show = false
  }
}

const actionMove = () => {
  modals.value.move.destPath = currentPath.value === '/' ? '/' : currentPath.value + '/'
  modals.value.move.show = true
  hideContextMenu()
}
const submitMove = async () => {
  const target = getFullFilePath(contextMenu.value.file.name)
  if (await doAction('move', target, modals.value.move.destPath)) {
    modals.value.move.show = false
  }
}

const actionCopy = () => {
  modals.value.copy.destPath = currentPath.value === '/' ? '/' : currentPath.value + '/'
  modals.value.copy.show = true
  hideContextMenu()
}
const submitCopy = async () => {
  const target = getFullFilePath(contextMenu.value.file.name)
  if (await doAction('copy', target, modals.value.copy.destPath)) {
    modals.value.copy.show = false
  }
}

const actionDelete = () => {
  const file = contextMenu.value.file
  hideContextMenu()
  showConfirm("Konfirmasi Hapus", `Hapus permanen ${file.name}? (Operasi tidak bisa dibatalkan)`, () => {
    doAction('delete', getFullFilePath(file.name))
  })
}

const actionInfo = async () => {
  const target = getFullFilePath(contextMenu.value.file.name)
  hideContextMenu()
  
  modals.value.info.show = true
  modals.value.info.isLoading = true
  modals.value.info.data = null
  
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/info?path=${encodeURIComponent(target)}`)
    const data = await res.json()
    if (res.ok) {
      modals.value.info.data = data
      modals.value.info.newPerms = data.permissions_octal
    } else throw new Error(data.error || "Cannot get file info")
  } catch (e) {
    showToast("Error", e.message, "error")
    modals.value.info.show = false
  } finally {
    modals.value.info.isLoading = false
  }
}

const submitChmod = async () => {
  if (!modals.value.info.data) return
  const target = getFullFilePath(modals.value.info.data.name)
  if (await doAction('chmod', target, modals.value.info.newPerms)) {
    modals.value.info.show = false
  }
}

const actionCompress = () => {
  modals.value.compress.zipName = contextMenu.value.file.name + '.zip'
  modals.value.compress.show = true
  hideContextMenu()
}
const submitCompress = async () => {
  const target = getFullFilePath(contextMenu.value.file.name)
  if (await doAction('compress', target, modals.value.compress.zipName)) {
    modals.value.compress.show = false
  }
}

const actionExtract = () => {
  modals.value.extract.destPath = currentPath.value
  modals.value.extract.password = ''
  modals.value.extract.show = true
  hideContextMenu()
}
const submitExtract = async () => {
  const target = getFullFilePath(contextMenu.value.file.name)
  if (await doAction('extract', target, modals.value.extract.destPath, modals.value.extract.password)) {
    modals.value.extract.show = false
  }
}

// ── TEXT EDITOR ──
const actionEditor = async () => {
  const file = contextMenu.value.file
  hideContextMenu()
  
  if (file.is_dir) return showToast("Warning", "Cannot open directory as text", "warning")
  if (file.size > 2 * 1024 * 1024) return showToast("Warning", "File is too large to edit (> 2MB)", "warning") // cegah lag

  modals.value.editor.show = true
  modals.value.editor.isLoading = true
  modals.value.editor.content = ''
  
  try {
    const target = getFullFilePath(file.name)
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/text`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: target })
    })
    const data = await res.json()
    if (res.ok) {
      modals.value.editor.content = data.content
      modals.value.editor.originalContent = data.content
    } else throw new Error(data.error || "Cannot read file")
  } catch (e) {
    showToast("Error", e.message, "error")
    modals.value.editor.show = false
  } finally {
    modals.value.editor.isLoading = false
  }
}

const submitEditor = async () => {
  const target = getFullFilePath(contextMenu.value.file.name)
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/files/text`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: target, content: modals.value.editor.content })
    })
    const data = await res.json()
    if (res.ok) {
      showToast("Success", "File saved successfully", "success")
      modals.value.editor.originalContent = modals.value.editor.content
    } else throw new Error(data.error || "Cannot save file")
  } catch (e) {
    showToast("Error", e.message, "error")
  }
}

onMounted(() => {
  fetchFiles('/')
  document.addEventListener('click', hideContextMenu)
})

onUnmounted(() => {
  document.removeEventListener('click', hideContextMenu)
})
</script>

<template>
  <section class="card h-[80vh] flex flex-col relative" @contextmenu.prevent="hideContextMenu">
    
    <!-- Top Action Bar -->
    <div class="flex justify-between items-center mb-4">
      <h2 class="card-title mb-0"><FolderTree class="w-5 h-5 text-brand-500" /> File Explorer</h2>
      <div class="flex gap-2">
        <button @click="promptFetch" class="btn-outline"><DownloadCloud class="w-4 h-4" /> Fetch</button>
        <label class="btn-outline cursor-pointer">
          <Upload class="w-4 h-4" /> Upload
          <input type="file" multiple class="hidden" @change="handleUpload">
        </label>
      </div>
    </div>
    
    <!-- Breadcrumb Nav -->
    <div class="flex items-center gap-3 mb-4 p-2 bg-slate-50 border border-slate-200 rounded shrink-0 dark:bg-slate-800 dark:border-slate-700">
      <button @click="navigateUp" class="btn-icon" title="Up Directory">
        <ArrowUp class="w-4 h-4" />
      </button>
      <div class="text-sm font-mono flex-1 overflow-x-hidden text-ellipsis whitespace-nowrap" :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ currentPath }}</div>
    </div>
    
    <!-- File List -->
    <div class="overflow-y-auto flex-1 border rounded-lg relative" :class="isDark ? 'border-slate-700 bg-slate-800' : 'border-slate-200 bg-white'">
      <table class="w-full relative">
        <thead class="sticky top-0 shadow-sm z-10" :class="isDark ? 'bg-slate-900' : 'bg-slate-50'">
          <tr>
            <th class="table-th" :class="isDark ? 'bg-slate-900' : 'bg-slate-50'"></th>
            <th class="table-th" :class="isDark ? 'bg-slate-900' : 'bg-slate-50'">Name</th>
            <th class="table-th" :class="isDark ? 'bg-slate-900' : 'bg-slate-50'">Size</th>
            <th class="table-th text-right" :class="isDark ? 'bg-slate-900' : 'bg-slate-50'">Action</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="f in files" :key="f.name" 
              class="select-none group transition-colors"
              :class="isDark ? 'hover:bg-slate-800/50' : 'hover:bg-slate-50'"
              @contextmenu.stop.prevent="showContextMenu($event, f)">
              
            <td class="table-td text-center border-b" :class="isDark ? 'border-slate-700' : 'border-slate-100'">
              <Folder v-if="f.is_dir" class="w-5 h-5 mx-auto" :class="isDark ? 'fill-blue-900/50 text-blue-400' : 'fill-blue-100 text-blue-500'" />
              <FileText v-else class="w-5 h-5 mx-auto" :class="isDark ? 'text-slate-500' : 'text-slate-400'" />
            </td>
            <td class="table-td font-medium cursor-pointer border-b" :class="isDark ? 'text-slate-300 group-hover:text-brand-400 border-slate-700' : 'text-slate-700 group-hover:text-brand-600 border-slate-100'" 
                @click="f.is_dir ? fetchFiles(getFullFilePath(f.name)) : null">
              <div class="flex flex-col leading-tight">
                <span>{{ f.name }}</span>
                <span class="text-[10px] font-normal mt-0.5" :class="isDark ? 'text-slate-500' : 'text-slate-400'">{{ new Date(f.modified * 1000).toLocaleString() }}</span>
              </div>
            </td>
            <td class="table-td text-xs border-b" :class="isDark ? 'text-slate-400 border-slate-700' : 'text-slate-500 border-slate-100'">{{ f.is_dir ? '-' : formatSize(f.size) }}</td>
            <td class="table-td text-right border-b" :class="isDark ? 'border-slate-700' : 'border-slate-100'">
              <div class="flex items-center justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                 <button @click="showContextMenu($event, f)" class="btn-icon" title="Menu"><MoreVertical class="w-3.5 h-3.5" /></button>
                 <a v-if="!f.is_dir" :href="`${getActiveServerUrl()}/api/files/download?path=${encodeURIComponent(getFullFilePath(f.name))}`" target="_blank" download class="btn-icon-primary" title="Download" @click.stop>
                   <Download class="w-3.5 h-3.5" />
                 </a>
              </div>
            </td>
          </tr>
          <tr v-if="files.length === 0">
            <td colspan="4" class="text-center p-8 text-sm" :class="isDark ? 'text-slate-500' : 'text-slate-500'">Folder is empty</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Custom Context Menu -->
    <div v-if="contextMenu.visible"
         class="fixed z-[100] border shadow-xl rounded-lg w-48 py-1 overflow-hidden"
         :class="isDark ? 'bg-slate-800 border-slate-700' : 'bg-white border-slate-200'"
         :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }"
         @click.stop>
      
      <div class="px-3 py-1.5 border-b mb-1 flex flex-col" :class="isDark ? 'border-slate-700' : 'border-slate-100'">
        <span class="text-xs font-semibold truncate" :class="isDark ? 'text-slate-200' : 'text-slate-800'">{{ contextMenu.file.name }}</span>
      </div>

      <button @click="actionRename" class="w-full text-left px-3 py-1.5 text-sm flex items-center gap-2" :class="isDark ? 'text-slate-300 hover:bg-slate-700' : 'text-slate-700 hover:bg-slate-100'">
        <Edit3 class="w-3.5 h-3.5" :class="isDark ? 'text-slate-400' : 'text-slate-400'" /> Rename
      </button>
      <button @click="actionMove" class="w-full text-left px-3 py-1.5 text-sm flex items-center gap-2" :class="isDark ? 'text-slate-300 hover:bg-slate-700' : 'text-slate-700 hover:bg-slate-100'">
        <Move class="w-3.5 h-3.5" :class="isDark ? 'text-slate-400' : 'text-slate-400'" /> Move
      </button>
      <button @click="actionCopy" class="w-full text-left px-3 py-1.5 text-sm flex items-center gap-2" :class="isDark ? 'text-slate-300 hover:bg-slate-700' : 'text-slate-700 hover:bg-slate-100'">
        <Copy class="w-3.5 h-3.5" :class="isDark ? 'text-slate-400' : 'text-slate-400'" /> Copy
      </button>
      <button @click="actionCompress" class="w-full text-left px-3 py-1.5 text-sm flex items-center gap-2" :class="isDark ? 'text-slate-300 hover:bg-slate-700' : 'text-slate-700 hover:bg-slate-100'">
        <Archive class="w-3.5 h-3.5" :class="isDark ? 'text-slate-400' : 'text-slate-400'" /> Compress to Zip
      </button>
      
      <button v-if="contextMenu.file.name.endsWith('.zip')" @click="actionExtract" class="w-full text-left px-3 py-1.5 text-sm flex items-center gap-2" :class="isDark ? 'text-slate-300 hover:bg-slate-700' : 'text-slate-700 hover:bg-slate-100'">
        <FolderTree class="w-3.5 h-3.5" :class="isDark ? 'text-slate-400' : 'text-slate-400'" /> Extract Zip
      </button>
      
      <button v-if="!contextMenu.file.is_dir" @click="actionEditor" class="w-full text-left px-3 py-1.5 text-sm flex items-center gap-2" :class="isDark ? 'text-slate-300 hover:bg-slate-700' : 'text-slate-700 hover:bg-slate-100'">
        <FileEdit class="w-3.5 h-3.5" :class="isDark ? 'text-slate-400' : 'text-slate-400'" /> Open as Text
      </button>

      <div class="h-px bg-slate-100 my-1"></div>

      <button @click="actionInfo" class="w-full text-left px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 flex items-center gap-2">
        <Info class="w-3.5 h-3.5 text-slate-400" /> Info &amp; Permissions
      </button>

      <button @click="actionDelete" class="w-full text-left px-3 py-1.5 text-sm text-red-600 hover:bg-red-50 flex items-center gap-2">
        <Trash2 class="w-3.5 h-3.5 text-red-400" /> Delete
      </button>
    </div>

  </section>

  <!-- ── MODALS (TELEPORTED) ── -->
  <Teleport to="body">
    
    <!-- Modal: Rename -->
    <div v-if="modals.rename.show" class="fixed inset-0 z-[110] bg-slate-900/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div class="bg-white rounded-xl shadow-xl w-full max-w-sm overflow-hidden">
        <div class="p-4 border-b border-slate-100 flex justify-between items-center bg-slate-50"><h3 class="font-bold">Rename</h3></div>
        <div class="p-5 space-y-4">
          <input v-model="modals.rename.newName" type="text" class="input-field" @keydown.enter="submitRename">
          <div class="flex justify-end gap-2"><button @click="modals.rename.show = false" class="btn-secondary">Cancel</button><button @click="submitRename" class="btn-primary">Rename</button></div>
        </div>
      </div>
    </div>

    <!-- Modal: Move -->
    <div v-if="modals.move.show" class="fixed inset-0 z-[110] bg-slate-900/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div class="bg-white rounded-xl shadow-xl w-full max-w-sm overflow-hidden">
        <div class="p-4 border-b border-slate-100 flex justify-between items-center bg-slate-50"><h3 class="font-bold">Move to...</h3></div>
        <div class="p-5 space-y-4">
          <input v-model="modals.move.destPath" type="text" class="input-field" placeholder="/home/user/target_dir" @keydown.enter="submitMove">
          <div class="flex justify-end gap-2"><button @click="modals.move.show = false" class="btn-secondary">Cancel</button><button @click="submitMove" class="btn-primary">Move</button></div>
        </div>
      </div>
    </div>

    <!-- Modal: Copy -->
    <div v-if="modals.copy.show" class="fixed inset-0 z-[110] backdrop-blur-sm flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
      <div class="rounded-xl shadow-xl w-full max-w-sm overflow-hidden" :class="isDark ? 'bg-slate-800' : 'bg-white'">
        <div class="p-4 border-b flex justify-between items-center" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'"><h3 class="font-bold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">Copy to...</h3></div>
        <div class="p-5 space-y-4">
          <input v-model="modals.copy.destPath" type="text" class="input-field" placeholder="/home/user/target_dir" @keydown.enter="submitCopy">
          <div class="flex justify-end gap-2"><button @click="modals.copy.show = false" class="btn-outline">Cancel</button><button @click="submitCopy" class="btn-primary">Copy</button></div>
        </div>
      </div>
    </div>

    <!-- Modal: Compress -->
    <div v-if="modals.compress.show" class="fixed inset-0 z-[110] backdrop-blur-sm flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
      <div class="rounded-xl shadow-xl w-full max-w-sm overflow-hidden" :class="isDark ? 'bg-slate-800' : 'bg-white'">
        <div class="p-4 border-b flex justify-between items-center" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'"><h3 class="font-bold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">Compress as Zip</h3></div>
        <div class="p-5 space-y-4">
          <label class="block text-xs font-semibold mb-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Archive Name</label>
          <input v-model="modals.compress.zipName" type="text" class="input-field" @keydown.enter="submitCompress">
          <div class="flex justify-end gap-2"><button @click="modals.compress.show = false" class="btn-outline">Cancel</button><button @click="submitCompress" class="btn-primary">Zip</button></div>
        </div>
      </div>
    </div>

    <!-- Modal: Extract -->
    <div v-if="modals.extract.show" class="fixed inset-0 z-[110] backdrop-blur-sm flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
      <div class="rounded-xl shadow-xl w-full max-w-sm overflow-hidden" :class="isDark ? 'bg-slate-800' : 'bg-white'">
        <div class="p-4 border-b flex justify-between items-center" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'"><h3 class="font-bold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">Extract Zip</h3></div>
        <div class="p-5 space-y-4">
          <div>
            <label class="block text-xs font-semibold mb-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Destination Directory</label>
            <input v-model="modals.extract.destPath" type="text" class="input-field" placeholder="e.g. /home/user/dir">
          </div>
          <div>
            <label class="block text-xs font-semibold mb-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Password (Optional)</label>
            <input v-model="modals.extract.password" type="password" class="input-field" placeholder="Leave empty if none">
          </div>
          <div class="flex justify-end gap-2"><button @click="modals.extract.show = false" class="btn-outline">Cancel</button><button @click="submitExtract" class="btn-primary">Extract</button></div>
        </div>
      </div>
    </div>

    <!-- Modal: Info & Chmod -->
    <div v-if="modals.info.show" class="fixed inset-0 z-[110] backdrop-blur-sm flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
      <div class="rounded-xl shadow-xl w-full max-w-sm overflow-hidden" :class="isDark ? 'bg-slate-800' : 'bg-white'">
        <div class="p-4 border-b flex justify-between items-center" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'"><h3 class="font-bold flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'"><Info class="w-4 h-4 text-blue-500" /> File Info</h3><button @click="modals.info.show = false" class="text-slate-400" :class="isDark ? 'hover:text-slate-200' : 'hover:text-slate-600'"><X class="w-4 h-4"/></button></div>
        
        <div class="p-5 flex flex-col justify-center items-center py-10" v-if="modals.info.isLoading">
          <Loader2 class="w-8 h-8 animate-spin text-brand-500 mb-2" />
          <p class="text-sm" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Reading metadata...</p>
        </div>
        
        <div class="p-5 space-y-4" v-else-if="modals.info.data">
          <div class="p-3 rounded-lg border space-y-2 text-sm" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'">
            <div class="flex items-start gap-2">
              <span class="font-medium w-16 shrink-0" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Name:</span>
              <span class="font-semibold break-all" :class="isDark ? 'text-slate-200' : 'text-slate-700'">{{ modals.info.data.name }}</span>
            </div>
            <div class="flex items-start gap-2">
              <span class="font-medium w-16 shrink-0" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Type:</span>
              <span :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ modals.info.data.is_dir ? 'Directory / Folder' : 'File' }}</span>
            </div>
            <div class="flex items-start gap-2" v-if="!modals.info.data.is_dir">
              <span class="font-medium w-16 shrink-0" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Size:</span>
              <span :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ formatSize(modals.info.data.size_bytes) }} <span class="text-xs" :class="isDark ? 'text-slate-500' : 'text-slate-400'">({{ modals.info.data.size_bytes }} bytes)</span></span>
            </div>
          </div>
          
          <div class="pt-2 border-t" :class="isDark ? 'border-slate-700' : 'border-slate-100'">
            <label class="block text-xs font-semibold mb-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Permissions (Octal)</label>
            <div class="flex gap-2">
              <input v-model="modals.info.newPerms" type="text" class="input-field font-mono w-full" placeholder="e.g. 0755" @keydown.enter="submitChmod">
              <button @click="submitChmod" class="btn-primary whitespace-nowrap">Apply</button>
            </div>
            <p class="text-[10px] mt-1" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Gunakan angka oktal standar Linux (misal 0644, 0755, 0777)</p>
          </div>
        </div>
      </div>
    </div>

    <!-- Modal: Editor -->
    <div v-if="modals.editor.show" class="fixed inset-0 z-[110] backdrop-blur-sm flex flex-col p-4 sm:p-8" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
      <div class="rounded-xl shadow-2xl w-full h-full flex flex-col overflow-hidden max-w-6xl mx-auto" :class="isDark ? 'bg-slate-900' : 'bg-white'">
        <div class="px-4 py-3 flex justify-between items-center shrink-0" :class="isDark ? 'bg-slate-950 border-b border-slate-800' : 'bg-slate-900'">
          <div class="flex items-center gap-2 text-white">
            <FileEdit class="w-4 h-4 text-blue-400" />
            <span class="font-mono text-sm">{{ contextMenu?.file?.name }}</span>
            <span v-if="modals.editor.originalContent !== modals.editor.content" class="w-2 h-2 rounded-full bg-amber-400"></span>
          </div>
          <div class="flex gap-2">
            <button @click="submitEditor" class="btn-primary !py-1 !text-xs" :disabled="modals.editor.originalContent === modals.editor.content"><Save class="w-3.5 h-3.5" /> Save</button>
            <button @click="modals.editor.show = false" class="p-1 rounded transition-colors" :class="isDark ? 'text-slate-400 hover:text-white bg-slate-800' : 'text-slate-400 hover:text-white bg-slate-800'"><X class="w-4 h-4"/></button>
          </div>
        </div>
        
        <div class="flex-1 relative p-1" :class="isDark ? 'bg-slate-900' : 'bg-slate-50'">
          <div v-if="modals.editor.isLoading" class="absolute inset-0 flex items-center justify-center z-10" :class="isDark ? 'bg-slate-900/80' : 'bg-white/80'">
            <Loader2 class="w-8 h-8 animate-spin text-blue-500" />
          </div>
          <textarea v-model="modals.editor.content" class="w-full h-full p-4 font-mono text-sm border rounded focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none shadow-inner leading-relaxed" :class="isDark ? 'bg-slate-950 text-slate-300 border-slate-800' : 'bg-white text-slate-800 border-slate-200'" spellcheck="false"></textarea>
        </div>
      </div>
    </div>

  </Teleport>
</template>
