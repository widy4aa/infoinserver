<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useServerStore } from '../stores/serverStore'
import { Box, Play, Square, RefreshCw, Trash2, FileText } from 'lucide-vue-next'
import { useToastStore } from '../stores/toastStore'

const { getActiveServerUrl } = useServerStore()
const { showConfirm, showToast } = useToastStore()
const containers = ref([])
const msg = ref('')
const isError = ref(false)

// Form Create
const cName = ref('')
const cImage = ref('')
const cPorts = ref('')

let pollInterval = null

const fetchContainers = async () => {
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/podman/containers`)
    if(res.ok) containers.value = await res.json()
  } catch (e) {}
}

const executeAction = async (action, id) => {
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/podman/containers/${action}/${id}`, { method: 'POST' })
    const data = await res.json()
    if (res.ok) {
      showToast("Success", data.message || `Container ${action} successful`, "success")
      fetchContainers()
    } else {
      showToast("Error", data || `Failed to ${action} container`, "error")
    }
  } catch (e) {
    showToast("Error", e.message, "error")
  }
}

const performAction = (action, id) => {
  if (action === 'rm') {
    showConfirm("Konfirmasi Hapus", `Hapus container ${id}?`, () => executeAction(action, id))
  } else {
    executeAction(action, id)
  }
}

const createContainer = async () => {
  if (!cName.value || !cImage.value) {
    msg.value = "Name & Image required"
    isError.value = true
    return
  }
  
  const portsArr = cPorts.value ? cPorts.value.split(',').map(s=>s.trim()) : []
  
  try {
    msg.value = 'Deploying...'
    isError.value = false
    const res = await fetch(`${getActiveServerUrl()}/api/podman/create`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: cName.value, image: cImage.value, ports: portsArr })
    })
    const data = await res.json()
    if(res.ok) {
      msg.value = data.message
      cName.value = ''; cImage.value = ''; cPorts.value = ''
      fetchContainers()
    } else {
      throw new Error(data)
    }
  } catch (e) {
    msg.value = e.message
    isError.value = true
  }
}

onMounted(() => {
  fetchContainers()
  pollInterval = setInterval(fetchContainers, 5000)
})

onUnmounted(() => {
  clearInterval(pollInterval)
})
</script>

<template>
  <section class="card">
    <h2 class="card-title"><Box class="w-5 h-5 text-brand-500" /> Podman Containers</h2>
    
    <div class="mb-5 p-4 bg-slate-50 border border-slate-200 rounded-lg">
      <h3 class="font-semibold text-sm mb-3">Deploy New Container</h3>
      <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
        <input v-model="cName" type="text" placeholder="Name (e.g. web)" class="input-field">
        <input v-model="cImage" type="text" placeholder="Image (e.g. nginx:alpine)" class="input-field">
        <input v-model="cPorts" type="text" placeholder="Ports (e.g. 8080:80)" class="input-field">
        <button @click="createContainer" class="btn-primary justify-center"><Play class="w-4 h-4" /> Deploy</button>
      </div>
      <div v-if="msg" class="mt-2 text-sm font-medium" :class="isError ? 'text-red-600' : 'text-green-600'">{{ msg }}</div>
    </div>

    <div class="overflow-x-auto">
      <table class="w-full">
        <thead>
          <tr>
            <th class="table-th">Name</th>
            <th class="table-th">Image</th>
            <th class="table-th">Status</th>
            <th class="table-th text-right">Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="c in containers" :key="c.Id">
            <td class="table-td font-medium">{{ c.Names?.[0] || c.Id.substring(0,12) }}</td>
            <td class="table-td text-slate-500 text-sm max-w-[200px] truncate" :title="c.Image">{{ c.Image }}</td>
            <td class="table-td">
              <div class="flex items-center gap-1.5">
                <span class="relative flex h-2.5 w-2.5">
                  <span v-if="c.State==='running'" class="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                  <span class="relative inline-flex rounded-full h-2.5 w-2.5" :class="c.State==='running' ? 'bg-green-500' : 'bg-slate-400'"></span>
                </span>
                <span class="text-xs capitalize" :class="c.State==='running' ? 'text-green-700' : 'text-slate-600'">{{ c.State }}</span>
              </div>
              <div class="text-[10px] text-slate-400 mt-1">{{ c.Status }}</div>
            </td>
            <td class="table-td text-right">
              <div class="flex items-center justify-end gap-1">
                <button @click="performAction('start', c.Id)" class="p-1 hover:bg-green-50 text-green-600 rounded disabled:opacity-30" :disabled="c.State==='running'" title="Start"><Play class="w-4 h-4" /></button>
                <button @click="performAction('stop', c.Id)" class="p-1 hover:bg-amber-50 text-amber-600 rounded disabled:opacity-30" :disabled="c.State!=='running'" title="Stop"><Square class="w-4 h-4" /></button>
                <button @click="performAction('restart', c.Id)" class="p-1 hover:bg-blue-50 text-blue-600 rounded" title="Restart"><RefreshCw class="w-4 h-4" /></button>
                <button @click="performAction('rm', c.Id)" class="p-1 hover:bg-red-50 text-red-600 rounded" title="Delete"><Trash2 class="w-4 h-4" /></button>
              </div>
            </td>
          </tr>
          <tr v-if="containers.length === 0">
            <td colspan="4" class="text-center p-4 text-slate-500">No containers found</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>