<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { Settings, Play, Square, RefreshCw, ToggleLeft, ToggleRight, Loader2, Search } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast, showConfirm } = useToastStore()

const services = ref([])
const processes = ref([])
const isLoadingServices = ref(true)

const searchQuery = ref('')
const filterActive = ref('all') // all, running, failed

const processSearchQuery = ref('')
const processSortBy = ref('cpu') // 'cpu' or 'ram'

let processPollInterval = null

// ── SERVICES ──
const fetchServices = async () => {
  isLoadingServices.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/services`)
    if (res.ok) {
      services.value = await res.json()
    } else {
      const err = await res.json()
      showToast("Error", err.error || "Failed to fetch services", "error")
    }
  } catch (e) {
    showToast("Error", "Network error", "error")
  } finally {
    isLoadingServices.value = false
  }
}

const filteredServices = computed(() => {
  let result = services.value

  if (filterActive.value === 'running') {
    result = result.filter(s => s.active === 'active')
  } else if (filterActive.value === 'failed') {
    result = result.filter(s => s.active === 'failed' || s.sub === 'failed')
  }

  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(s => s.unit.toLowerCase().includes(q) || s.description.toLowerCase().includes(q))
  }

  return result
})

const handleAction = async (action, serviceName) => {
  showToast("Info", `${action}ing ${serviceName}...`)
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/services/action`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action, service_name: serviceName })
    })
    const data = await res.json()
    if (res.ok) {
      showToast("Success", data.message, "success")
      fetchServices() // refresh list
    } else {
      showToast("Error", data.error || "Action failed", "error")
    }
  } catch (e) {
    showToast("Error", e.message, "error")
  }
}

// ── PROCESSES ──
const fetchProcesses = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/process/list`)
    if (res.ok) {
      processes.value = await res.json()
    }
  } catch (err) {
    console.error(err)
  }
}

const filteredAndSortedProcesses = computed(() => {
  let result = [...processes.value]
  
  if (processSearchQuery.value) {
    const q = processSearchQuery.value.toLowerCase()
    result = result.filter(p => p.name.toLowerCase().includes(q) || p.pid.toString().includes(q))
  }
  
  if (processSortBy.value === 'cpu') {
    result.sort((a, b) => b.cpu_usage - a.cpu_usage)
  } else if (processSortBy.value === 'ram') {
    result.sort((a, b) => b.memory_bytes - a.memory_bytes)
  }
  
  return result
})

const killProcess = (pid) => {
  showConfirm("Konfirmasi", `Kill process PID ${pid}?`, async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/process/kill/${pid}`, { method: 'POST' })
      if(res.ok) {
        fetchProcesses()
        showToast("Success", `Process ${pid} killed`, "success")
      } else {
        showToast("Error", "Failed to kill process", "error")
      }
    } catch(e) {
      showToast("Error", e.message, "error")
    }
  })
}

onMounted(() => {
  fetchServices()
  fetchProcesses()
  // Poll processes tiap 3 detik karena ini tidak pakai ws
  processPollInterval = setInterval(fetchProcesses, 3000)
})

onUnmounted(() => {
  clearInterval(processPollInterval)
})
</script>

<template>
  <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
    
    <!-- SERVICES TABLE (Kiri) -->
    <section class="card h-[calc(100vh-12rem)] flex flex-col lg:col-span-2">
      <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-4 gap-4 shrink-0">
        <h2 class="card-title mb-0"><Settings class="w-5 h-5 text-brand-500" /> Systemd Services</h2>
        <div class="flex gap-2 w-full sm:w-auto">
          <div class="relative flex-1 sm:w-64">
            <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400 pointer-events-none" />
            <input v-model="searchQuery" type="text" placeholder="Search service..." class="input-field !pl-9">
          </div>
          <select v-model="filterActive" class="input-field w-auto">
            <option value="all">All</option>
            <option value="running">Running</option>
            <option value="failed">Failed</option>
          </select>
          <button @click="fetchServices" class="btn-outline px-3" title="Refresh">
            <RefreshCw class="w-4 h-4" />
          </button>
        </div>
      </div>

      <div v-if="isLoadingServices" class="flex justify-center p-12">
        <Loader2 class="w-8 h-8 animate-spin text-brand-500" />
      </div>

      <div v-else class="overflow-y-auto overflow-x-hidden flex-1 border border-slate-200 rounded-lg bg-white relative">
        <table class="w-full relative">
          <thead class="sticky top-0 bg-slate-50 shadow-sm z-10">
            <tr>
              <th class="table-th w-1/2">Unit Name</th>
              <th class="table-th w-24">Status</th>
              <th class="table-th text-right w-40 sticky right-0 bg-slate-50 shadow-[-4px_0_6px_-2px_rgba(0,0,0,0.04)]">Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="s in filteredServices" :key="s.unit" class="hover:bg-slate-50 group">
              <td class="table-td w-1/2 max-w-0">
                <div class="truncate font-mono text-sm text-slate-800 font-semibold">{{ s.unit }}</div>
                <div class="truncate text-xs text-slate-400 mt-0.5" :title="s.description">{{ s.description }}</div>
              </td>
              <td class="table-td">
                <span class="px-2 py-0.5 rounded text-[10px] font-bold uppercase"
                      :class="{
                        'bg-green-100 text-green-700': s.active === 'active',
                        'bg-red-100 text-red-700': s.active === 'failed' || s.sub === 'failed',
                        'bg-slate-100 text-slate-600': s.active !== 'active' && s.active !== 'failed'
                      }">
                  {{ s.active }}
                </span>
                <div class="text-[10px] text-slate-400 mt-1 capitalize">{{ s.sub }}</div>
              </td>
              <td class="table-td text-right sticky right-0 bg-white dark:bg-slate-800 shadow-[-4px_0_6px_-2px_rgba(0,0,0,0.04)]">
                <div class="flex items-center justify-end gap-1">
                  <button @click="handleAction('start', s.unit)" class="btn-icon-green" title="Start" :disabled="s.active === 'active'"><Play class="w-3 h-3" /></button>
                  <button @click="handleAction('stop', s.unit)" class="btn-icon-amber" title="Stop" :disabled="s.active !== 'active'"><Square class="w-3 h-3" /></button>
                  <button @click="handleAction('restart', s.unit)" class="btn-icon-blue" title="Restart"><RefreshCw class="w-3 h-3" /></button>
                  
                  <div class="w-px h-6 bg-slate-200 mx-1"></div>
                  
                  <button v-if="s.load === 'loaded' || s.load === 'enabled'" @click="handleAction('disable', s.unit)" class="btn-icon" title="Disable (Auto-start OFF)"><ToggleRight class="w-4 h-4 text-green-500" /></button>
                  <button v-else @click="handleAction('enable', s.unit)" class="btn-icon" title="Enable (Auto-start ON)"><ToggleLeft class="w-4 h-4 text-slate-400" /></button>
                </div>
              </td>
            </tr>
            <tr v-if="filteredServices.length === 0">
              <td colspan="3" class="text-center p-8 text-slate-500">No services found</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- PROCESSES PANEL (Kanan) -->
    <section class="lg:col-span-1">
      <div class="card flex flex-col h-full max-h-[calc(100vh-12rem)]">
        <div class="flex items-center justify-between shrink-0 mb-3">
          <h2 class="card-title mb-0">Top Processes</h2>
          
          <div class="flex items-center gap-1 bg-slate-100 p-0.5 rounded-md">
            <button @click="processSortBy = 'cpu'" :class="processSortBy === 'cpu' ? 'bg-white shadow-sm text-slate-800' : 'text-slate-500 hover:text-slate-700'" class="px-2 py-1 rounded text-[10px] font-bold uppercase transition-all">CPU</button>
            <button @click="processSortBy = 'ram'" :class="processSortBy === 'ram' ? 'bg-white shadow-sm text-slate-800' : 'text-slate-500 hover:text-slate-700'" class="px-2 py-1 rounded text-[10px] font-bold uppercase transition-all">RAM</button>
          </div>
        </div>

        <div class="relative shrink-0 mb-3">
          <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400 pointer-events-none" />
          <input v-model="processSearchQuery" type="text" placeholder="Search PID or Name..." class="input-field !pl-9">
        </div>

        <div class="overflow-y-auto flex-1 pr-1 space-y-1.5 -mr-1">
          <div v-for="p in filteredAndSortedProcesses" :key="p.pid" class="flex items-center justify-between p-2 bg-slate-50 hover:bg-slate-100 rounded-lg border border-slate-100 transition-colors">
            <div class="flex-1 min-w-0 mr-2">
              <div class="font-semibold text-[13px] text-slate-700 truncate" :title="p.name">{{ p.name }}</div>
              <div class="text-[10px] text-slate-400 font-mono mt-0.5">PID: {{ p.pid }}</div>
            </div>
            <div class="flex flex-col items-end mr-3 w-16">
              <span class="text-xs font-bold" :class="processSortBy === 'cpu' ? 'text-brand-600' : 'text-slate-600'">{{ p.cpu_usage.toFixed(1) }}%</span>
              <span class="text-[10px]" :class="processSortBy === 'ram' ? 'text-purple-600 font-bold' : 'text-slate-500'">{{ (p.memory_bytes/1048576).toFixed(1) }} MB</span>
            </div>
            <button @click="killProcess(p.pid)" class="btn-icon-red shrink-0 w-7 h-7" title="Kill Process">
              <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
            </button>
          </div>
          
          <div v-if="filteredAndSortedProcesses.length === 0" class="text-center py-6 text-slate-400 text-sm">
            Loading processes...
          </div>
        </div>
      </div>
    </section>

  </div>
</template>
