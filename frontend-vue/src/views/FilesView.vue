<script setup>
import { ref, onMounted } from 'vue'
import { useServerStore } from '../stores/serverStore'
import { FolderTree, DownloadCloud, Upload, ArrowUp, Folder, FileText, Download } from 'lucide-vue-next'

const { getActiveServerUrl } = useServerStore()

const files = ref([])
const currentPath = ref('/')
const msg = ref('')
const isError = ref(false)

const fetchFiles = async (path) => {
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/files/list?path=${encodeURIComponent(path)}`)
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

const handleUpload = async (event) => {
  const fileList = event.target.files
  if(!fileList || fileList.length === 0) return
  
  msg.value = `Uploading ${fileList.length} file(s)...`
  isError.value = false
  
  const fd = new FormData()
  for(let i=0; i<fileList.length; i++) fd.append('file', fileList[i])
    
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/files/upload?path=${encodeURIComponent(currentPath.value)}`, {
      method: 'POST',
      body: fd
    })
    const data = await res.json()
    if(res.ok) {
      msg.value = data.message
      fetchFiles(currentPath.value)
    } else {
      throw new Error(data)
    }
  } catch (e) {
    msg.value = e.message
    isError.value = true
  } finally {
    event.target.value = ''
    setTimeout(()=>msg.value='', 5000)
  }
}

const promptFetch = async () => {
  const url = prompt("Masukkan URL file untuk di-fetch (wget):")
  if(!url) return
  
  msg.value = "Fetching from URL..."
  isError.value = false
  
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/files/fetch`, {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({ url, path: currentPath.value })
    })
    const data = await res.json()
    if(res.ok) {
      msg.value = data.message
      fetchFiles(currentPath.value)
    } else throw new Error(data)
  } catch(e) {
    msg.value = e.message
    isError.value = true
  } finally {
    setTimeout(()=>msg.value='', 5000)
  }
}

const formatSize = (bytes) => {
  if (bytes === 0) return '0 B'
  const k = 1024, dm = 2, sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i]
}

onMounted(() => fetchFiles('/'))
</script>

<template>
  <section class="card h-[80vh] flex flex-col">
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
    
    <div v-if="msg" class="text-sm font-medium mb-2" :class="isError ? 'text-red-600' : 'text-green-600'">{{ msg }}</div>
    
    <div class="flex items-center gap-3 mb-4 p-2 bg-slate-50 rounded border border-slate-200 shrink-0">
      <button @click="navigateUp" class="p-1.5 hover:bg-slate-200 rounded text-slate-600" title="Up Directory">
        <ArrowUp class="w-5 h-5" />
      </button>
      <div class="text-sm font-mono text-slate-700 flex-1 overflow-x-hidden text-ellipsis whitespace-nowrap">{{ currentPath }}</div>
    </div>
    
    <div class="overflow-y-auto flex-1 border border-slate-200 rounded-lg">
      <table class="w-full relative">
        <thead class="sticky top-0 bg-slate-50 shadow-sm z-10">
          <tr>
            <th class="table-th w-10"></th>
            <th class="table-th">Name</th>
            <th class="table-th">Size</th>
            <th class="table-th text-right">Action</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="f in files" :key="f.name" class="hover:bg-slate-50">
            <td class="table-td text-center">
              <Folder v-if="f.is_dir" class="w-5 h-5 mx-auto fill-blue-100 text-blue-500" />
              <FileText v-else class="w-5 h-5 mx-auto text-slate-400" />
            </td>
            <td class="table-td font-medium cursor-pointer text-slate-700 hover:text-brand-600" @click="f.is_dir ? fetchFiles(currentPath==='/' ? '/'+f.name : currentPath+'/'+f.name) : null">
              <div class="flex flex-col leading-tight">
                <span>{{ f.name }}</span>
                <span class="text-[10px] text-slate-400 font-normal mt-0.5">{{ new Date(f.modified * 1000).toLocaleString() }}</span>
              </div>
            </td>
            <td class="table-td text-slate-500 text-xs">{{ f.is_dir ? '-' : formatSize(f.size) }}</td>
            <td class="table-td text-right">
              <a v-if="!f.is_dir" :href="`${getActiveServerUrl()}/api/files/download?path=${encodeURIComponent(currentPath==='/' ? '/'+f.name : currentPath+'/'+f.name)}`" target="_blank" download class="p-1.5 inline-flex hover:bg-brand-50 text-brand-600 rounded">
                <Download class="w-4 h-4" />
              </a>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>