<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { Cpu, Loader2 } from 'lucide-vue-next'

const { getActiveServerUrl } = useServerStore()
const { showConfirm, showToast } = useToastStore()

const sysInfo = ref(null)
const processes = ref([])
const error = ref(null)
let pollInterval = null

const fetchSysInfo = async () => {
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/system`)
    if (!res.ok) throw new Error("Failed to fetch system info")
    sysInfo.value = await res.json()
    error.value = null
  } catch (err) {
    error.value = err.message
  }
}

const fetchProcesses = async () => {
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/process/list`)
    if (!res.ok) throw new Error("Failed to fetch processes")
    processes.value = await res.json()
  } catch (err) {
    console.error(err)
  }
}

const killProcess = (pid) => {
  showConfirm("Konfirmasi", `Kill process PID ${pid}?`, async () => {
    try {
      const res = await fetch(`${getActiveServerUrl()}/api/process/kill/${pid}`, { method: 'POST' })
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

const formatUptime = (seconds) => {
  if (!seconds) return '0m'
  const d = Math.floor(seconds / 86400)
  const h = Math.floor((seconds % 86400) / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return `${d}d ${h}h ${m}m`
}

onMounted(() => {
  fetchSysInfo()
  fetchProcesses()
  pollInterval = setInterval(() => {
    fetchSysInfo()
    fetchProcesses()
  }, 3000)
})

onUnmounted(() => {
  clearInterval(pollInterval)
})
</script>

<template>
  <div class="space-y-6">
    <div v-if="error" class="bg-red-50 text-red-600 p-4 rounded-md border border-red-200">
      Error connecting to backend: {{ error }}. Check Settings tab.
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- System Info -->
      <section class="card lg:col-span-2">
        <h2 class="card-title"><Cpu class="w-5 h-5 text-brand-500" /> System Resources</h2>
        
        <div v-if="!sysInfo" class="flex items-center gap-2 text-slate-500 text-sm">
          <Loader2 class="w-4 h-4 animate-spin" /> Loading metrics...
        </div>

        <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
          <div class="p-4 bg-slate-50 rounded-lg border border-slate-100">
            <div class="text-sm font-medium text-slate-500 mb-1">Hostname & OS</div>
            <div class="font-semibold text-slate-800">{{ sysInfo.hostname }}</div>
            <div class="text-xs text-slate-500 mt-1">{{ sysInfo.os_name }} • {{ sysInfo.kernel_version }}</div>
          </div>
          
          <div class="p-4 bg-green-50 rounded-lg border border-green-100">
            <div class="text-sm font-medium text-green-600 mb-1">Uptime</div>
            <div class="font-bold text-green-700 text-lg">{{ formatUptime(sysInfo.uptime) }}</div>
          </div>
          
          <div class="p-4 bg-brand-50 rounded-lg border border-brand-100 md:col-span-2">
            <div class="flex justify-between items-center mb-2">
              <div class="text-sm font-medium text-brand-600">CPU Usage ({{ sysInfo.cpu_cores }} Cores)</div>
              <div class="text-xs text-brand-600 font-mono truncate max-w-[150px]" :title="sysInfo.cpu_model">{{ sysInfo.cpu_model }}</div>
            </div>
            <div class="flex items-center gap-3">
              <div class="text-2xl font-bold text-brand-700 w-20">{{ sysInfo.global_cpu_usage?.toFixed(1) }}%</div>
              <div class="flex-1 bg-brand-200 rounded-full h-2.5 overflow-hidden">
                <div class="bg-brand-500 h-2.5 rounded-full transition-all duration-300" :style="`width: ${Math.min(Math.max(sysInfo.global_cpu_usage||0, 0), 100)}%`"></div>
              </div>
            </div>
          </div>
          
          <div class="p-4 bg-purple-50 rounded-lg border border-purple-100 md:col-span-2">
            <div class="text-sm font-medium text-purple-600 mb-2">Memory (RAM)</div>
            <div class="flex items-center gap-3">
              <div class="font-bold text-purple-700 w-32 whitespace-nowrap text-sm">
                {{ (sysInfo.used_memory / 1024 / 1024 / 1024).toFixed(2) }} GB / {{ (sysInfo.total_memory / 1024 / 1024 / 1024).toFixed(2) }} GB
              </div>
              <div class="flex-1 bg-purple-200 rounded-full h-2.5 overflow-hidden">
                <div class="bg-purple-500 h-2.5 rounded-full transition-all duration-300" :style="`width: ${Math.min(sysInfo.used_memory/sysInfo.total_memory*100, 100)}%`"></div>
              </div>
            </div>
          </div>
          
          <div class="p-4 bg-amber-50 rounded-lg border border-amber-100 md:col-span-2">
            <div class="text-sm font-medium text-amber-600 mb-2">Storage (Disks)</div>
            <div class="space-y-3 max-h-48 overflow-y-auto pr-1">
              <div v-for="disk in sysInfo.disks" :key="disk.mount_point" class="flex flex-col">
                <div class="flex justify-between items-center text-xs mb-1" v-if="disk.total_space > 0">
                  <span class="font-medium text-amber-900 truncate flex-1 pr-2" :title="disk.mount_point">
                    {{ disk.mount_point }} <span class="text-amber-600 font-normal">({{ disk.name }})</span>
                  </span>
                  <span class="text-amber-700 whitespace-nowrap">
                    {{ ((disk.total_space - disk.available_space)/1073741824).toFixed(1) }} GB / {{ (disk.total_space/1073741824).toFixed(1) }} GB
                  </span>
                </div>
                <div class="w-full bg-amber-200 rounded-full h-2 overflow-hidden" v-if="disk.total_space > 0">
                  <div class="bg-amber-500 h-full rounded-full transition-all duration-300" :style="`width: ${Math.min(((disk.total_space-disk.available_space)/disk.total_space)*100, 100)}%`"></div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- Task Manager -->
      <section class="card lg:col-span-1 flex flex-col max-h-[600px]">
        <h2 class="card-title">Top Processes</h2>
        <div class="overflow-y-auto flex-1 pr-2 space-y-2">
          <div v-for="p in processes" :key="p.pid" class="flex items-center justify-between p-2 bg-slate-50 hover:bg-slate-100 rounded border border-slate-100 transition-colors">
            <div class="flex-1 min-w-0 mr-3">
              <div class="font-medium text-sm text-slate-700 truncate" :title="p.name">{{ p.name }}</div>
              <div class="text-[10px] text-slate-500 font-mono mt-0.5">PID: {{ p.pid }}</div>
            </div>
            <div class="flex flex-col items-end mr-3 w-16">
              <span class="text-xs font-bold text-brand-600">{{ p.cpu_usage.toFixed(1) }}%</span>
              <span class="text-[10px] text-purple-600">{{ (p.memory_bytes/1048576).toFixed(1) }} MB</span>
            </div>
            <button @click="killProcess(p.pid)" class="p-1.5 text-red-400 hover:text-red-600 hover:bg-red-50 rounded transition-colors" title="Kill Process">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/><path d="M8 20v2h8v-2"/><path d="m12.5 17-.5-1-.5 1h1z"/><path d="M16 20a2 2 0 0 0 1.56-3.25 8 8 0 1 0-11.12 0A2 2 0 0 0 8 20"/></svg>
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>