<script setup>
import { ref } from 'vue'
import { useRoute } from 'vue-router'
import { useServerStore } from '../stores/serverStore'
import { useRouter } from 'vue-router'
import { Settings, Server, Plus, Trash2, Power, RefreshCw, Edit2 } from 'lucide-vue-next'

const route = useRoute()
const router = useRouter()
const { servers, activeServerId, addServer, removeServer, setActiveServer, getActiveServerUrl, updateServerName } = useServerStore()

const newName = ref('')
const newUrl = ref('')

const handleAdd = () => {
  if(newName.value && newUrl.value) {
    addServer(newName.value, newUrl.value)
    newName.value = ''
    newUrl.value = ''
    alert("Server added! Go back to Home to see it.")
  }
}

const handleRenameServer = () => {
  const currentName = servers.value.find(s => s.id === route.params.id)?.name || ''
  const newNameStr = prompt("Masukkan nama baru untuk server ini:", currentName)
  if (newNameStr !== null && newNameStr.trim() !== '') {
    updateServerName(route.params.id, newNameStr.trim())
  }
}

const handleRemoveServer = () => {
  if(confirm("Apakah Anda yakin ingin menghapus server ini dari dashboard?")) {
    removeServer(route.params.id)
    router.push('/')
  }
}

const handleUpdate = async () => {
  if(!confirm("Pull changes and rebuild dashboard?")) return
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/system/update`, {method: 'POST'})
    const data = await res.json()
    alert(res.ok ? data.message : data)
  } catch(e) {
    alert(e.message)
  }
}

const handleReboot = async () => {
  if(!confirm("DANGER: Reboot Host OS physically?")) return
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/system/reboot`, {method: 'POST'})
    const data = await res.json()
    alert(res.ok ? data.message : data)
  } catch(e) {
    alert(e.message)
  }
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
        <div class="flex flex-col md:flex-row gap-3">
          <input v-model="newName" type="text" placeholder="Server Name (e.g. VPS Singapore)" class="input-field md:w-1/3 text-slate-900 placeholder:text-slate-400">
          <input v-model="newUrl" type="text" placeholder="http://<IP>:8080" class="input-field md:flex-1 w-full text-slate-900 placeholder:text-slate-400">
          <button @click="handleAdd" class="btn-primary px-6 whitespace-nowrap"><Plus class="w-4 h-4" /> Add Server</button>
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