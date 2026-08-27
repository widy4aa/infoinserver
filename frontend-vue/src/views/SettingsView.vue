<script setup>
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { Settings, Server, Plus, Trash2, Power, RefreshCw, Edit2, Play } from 'lucide-vue-next'

const route = useRoute()
const router = useRouter()
const { servers, activeServerId, addServer, removeServer, setActiveServer, getActiveServerUrl, updateServerName } = useServerStore()
const { showToast, showConfirm } = useToastStore()

const newName = ref('')
const newUrl = ref('')
const isTesting = ref(false)

const handleTest = async () => {
  if(!newUrl.value) {
    showToast("Info", "Please enter a URL first", "warning")
    return
  }
  
  isTesting.value = true
  const cleanUrl = newUrl.value.endsWith('/') ? newUrl.value.slice(0, -1) : newUrl.value;
  
  try {
    const res = await fetch(`${cleanUrl}/api/system`, { method: 'GET' })
    if (res.ok) {
      const data = await res.json()
      showToast("Connection Successful", `Hostname: ${data.hostname || 'Unknown'}`, "success")
    } else {
      showToast("Error", `Received status code ${res.status}`, "error")
    }
  } catch(e) {
    showToast("Connection Failed", `${e.message}\nMake sure backend is running and CORS is enabled.`, "error")
  } finally {
    isTesting.value = false
  }
}

const handleAdd = () => {
  if(newName.value && newUrl.value) {
    addServer(newName.value, newUrl.value)
    newName.value = ''
    newUrl.value = ''
    showToast("Success", "Server added! Go back to Home to see it.", "success")
  } else {
    showToast("Warning", "Please fill both Name and URL", "warning")
  }
}

const handleRenameServer = () => {
  const currentName = servers.value.find(s => s.id === route.params.id)?.name || ''
  const newNameStr = prompt("Masukkan nama baru untuk server ini:", currentName)
  if (newNameStr !== null && newNameStr.trim() !== '') {
    updateServerName(route.params.id, newNameStr.trim())
    showToast("Success", "Server renamed successfully", "success")
  }
}

const handleRemoveServer = () => {
  showConfirm(
    "Remove Server", 
    "Apakah Anda yakin ingin menghapus server ini dari dashboard?",
    () => {
      removeServer(route.params.id)
      router.push('/')
      showToast("Removed", "Server has been removed from dashboard", "info")
    }
  )
}

const handleUpdate = () => {
  showConfirm(
    "Update Dashboard", 
    "Pull changes and rebuild dashboard backend remotely?",
    async () => {
      try {
        const res = await fetch(`${getActiveServerUrl()}/api/system/update`, {method: 'POST'})
        const data = await res.json()
        if(res.ok) {
           showToast("Success", data.message, "success")
        } else {
           showToast("Error", data, "error")
        }
      } catch(e) {
        showToast("Failed", e.message, "error")
      }
    }
  )
}

const handleReboot = () => {
  showConfirm(
    "DANGER: Reboot OS", 
    "Reboot Host OS physically? This will disrupt all services.",
    async () => {
      try {
        const res = await fetch(`${getActiveServerUrl()}/api/system/reboot`, {method: 'POST'})
        const data = await res.json()
        if(res.ok) {
           showToast("Rebooting", data.message, "warning")
        } else {
           showToast("Error", data, "error")
        }
      } catch(e) {
        showToast("Failed", e.message, "error")
      }
    }
  )
}
</script>

<template>
  <div class="space-y-6">
    <!-- Server Management (Global) -->
    <section class="card" v-if="!route.params.id">
      <h2 class="card-title"><Settings class="w-5 h-5 text-brand-500" /> Global Configuration</h2>
      <p class="text-sm text-slate-500 mb-6">Add backend servers to monitor here. They will appear on your Home screen.</p>
      
      <div class="p-5 border border-slate-200 bg-slate-50 rounded-lg">
        <h3 class="text-sm font-semibold mb-3">Add New Backend Node</h3>
        <div class="flex flex-col gap-4 mt-4">
          <div class="flex flex-col gap-1">
            <label class="text-sm font-medium text-slate-700">Server Name / Alias</label>
            <input v-model="newName" type="text" placeholder="e.g. VPS Singapore" class="input-field text-slate-900 placeholder:text-slate-400 w-full">
          </div>
          <div class="flex flex-col gap-1">
            <label class="text-sm font-medium text-slate-700">Backend URL (IP & Port)</label>
            <input v-model="newUrl" type="text" placeholder="http://<IP>:8080" class="input-field text-slate-900 placeholder:text-slate-400 w-full">
          </div>
          <div class="flex gap-2 mt-2">
            <button @click="handleTest" class="btn-outline whitespace-nowrap" :disabled="isTesting">
              <Play class="w-4 h-4" /> {{ isTesting ? 'Testing...' : 'Test Connection' }}
            </button>
            <button @click="handleAdd" class="btn-primary whitespace-nowrap"><Plus class="w-4 h-4" /> Add Server</button>
          </div>
        </div>
      </div>
    </section>

    <!-- Server Settings (Server specific) -->
    <section class="card" v-if="route.params.id">
      <h2 class="card-title"><Settings class="w-5 h-5 text-brand-500" /> Server Preferences</h2>
      
      <div class="space-y-4 mb-6">
        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border border-slate-200 rounded-lg bg-slate-50 gap-4">
          <div>
            <div class="font-medium text-slate-800">Rename Server Alias</div>
            <div class="text-xs text-slate-500">Change how this server appears on the home screen</div>
          </div>
          <button @click="handleRenameServer" class="btn-outline text-brand-600 border-brand-200 hover:bg-brand-50 whitespace-nowrap">
            <Edit2 class="w-4 h-4" /> Rename
          </button>
        </div>

        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border border-slate-200 rounded-lg bg-slate-50 gap-4">
          <div>
            <div class="font-medium text-slate-800">Remove Server</div>
            <div class="text-xs text-slate-500">Remove this server from your dashboard list</div>
          </div>
          <button @click="handleRemoveServer" class="btn-outline text-red-600 border-red-200 hover:bg-red-50 whitespace-nowrap">
            <Trash2 class="w-4 h-4" /> Remove
          </button>
        </div>
      </div>
    </section>

    <!-- Danger Zone (Server specific) -->
    <section class="card border-red-200" v-if="route.params.id">
      <h2 class="card-title text-red-600"><Power class="w-5 h-5" /> Danger Zone</h2>
      <p class="text-sm text-slate-500 mb-6">Actions below affect this specific backend server OS ({{ getActiveServerUrl() }}).</p>
      
      <div class="space-y-4">
        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border border-slate-200 rounded-lg bg-slate-50 gap-4">
          <div>
            <div class="font-medium text-slate-800">Update Dashboard Backend</div>
            <div class="text-xs text-slate-500">Run git pull & cargo build --release remotely</div>
          </div>
          <button @click="handleUpdate" class="btn-outline text-brand-600 border-brand-200 hover:bg-brand-50 whitespace-nowrap"><RefreshCw class="w-4 h-4" /> Update Backend</button>
        </div>
        
        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border border-red-200 bg-red-50 rounded-lg gap-4">
          <div>
            <div class="font-medium text-red-800">Reboot Host</div>
            <div class="text-xs text-red-600">Reboot the physical operating system</div>
          </div>
          <button @click="handleReboot" class="btn-destructive whitespace-nowrap"><Power class="w-4 h-4" /> Reboot Server</button>
        </div>
      </div>
    </section>
  </div>
</template>