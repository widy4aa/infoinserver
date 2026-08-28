<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { Cpu, Loader2, Activity, Clock, Search, ArrowDownUp } from 'lucide-vue-next'

import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler
} from 'chart.js'
import { Line } from 'vue-chartjs'

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler
)

const { apiFetch } = useApi()
const { getActiveServerUrl, getToken, activeServerId } = useServerStore()
const { showConfirm, showToast } = useToastStore()

const sysInfo = ref(null)
const processes = ref([])
const error = ref(null)
let ws = null

// Process Filtering & Sorting
const processSearchQuery = ref('')
const processSortBy = ref('cpu') // 'cpu' or 'ram'

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

// History Data
const historyData = ref([])
const fullHistoryData = ref([]) // Menyimpan data utuh dari API
const historyTimeRange = ref('24h') // '24h', '12h', '1h', '30m', '10m', '5m'

const filterHistoryByTime = (range) => {
  if (!fullHistoryData.value || fullHistoryData.value.length === 0) return []
  
  const now = new Date()
  let limitTime = new Date()
  
  switch(range) {
    case '5m': limitTime.setMinutes(now.getMinutes() - 5); break;
    case '10m': limitTime.setMinutes(now.getMinutes() - 10); break;
    case '30m': limitTime.setMinutes(now.getMinutes() - 30); break;
    case '1h': limitTime.setHours(now.getHours() - 1); break;
    case '12h': limitTime.setHours(now.getHours() - 12); break;
    case '24h': default: limitTime.setHours(now.getHours() - 24); break;
  }
  
  return fullHistoryData.value.filter(d => new Date(d.timestamp) >= limitTime)
}

const setTimeRange = (range) => {
  historyTimeRange.value = range
  historyData.value = filterHistoryByTime(range)
}

const connectWebSocket = () => {
  const token = getToken(activeServerId.value)
  if (!token) return

  const wsUrl = getActiveServerUrl().replace(/^http/, 'ws') + `/api/metrics/ws?token=${token}`
  ws = new WebSocket(wsUrl)

  ws.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data)
      if (data.type === 'metrics_update') {
        sysInfo.value = data.system
        processes.value = data.processes
        error.value = null
      }
    } catch (e) {
      console.error('Failed to parse WS data', e)
    }
  }

  ws.onerror = () => {
    error.value = "WebSocket connection error. Check backend or auth."
  }

  ws.onclose = () => {
    // Reconnect hanya jika ws masih merujuk ke instance yang sama (tidak di-null-kan oleh unmount)
    if (ws) {
      setTimeout(() => {
        if (ws) connectWebSocket()
      }, 3000)
    }
  }
}

const fetchHistory = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/metrics/history`)
    if (res.ok) {
      fullHistoryData.value = await res.json()
      // Filter dengan range aktif
      historyData.value = filterHistoryByTime(historyTimeRange.value)
    }
  } catch (e) {
    console.error("Failed to fetch metrics history", e)
  }
}

const killProcess = (pid) => {
  showConfirm("Konfirmasi", `Kill process PID ${pid}?`, async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/process/kill/${pid}`, { method: 'POST' })
      if(res.ok) {
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

// ── Chart Configurations ──
const chartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  interaction: {
    mode: 'index',
    intersect: false,
  },
  plugins: {
    legend: { display: false },
    tooltip: {
      backgroundColor: 'rgba(15, 23, 42, 0.9)',
      titleFont: { size: 11 },
      bodyFont: { size: 12 },
      padding: 10,
      cornerRadius: 6,
    }
  },
  scales: {
    x: {
      grid: { display: false },
      ticks: {
        maxTicksLimit: 6,
        font: { size: 10 },
        color: '#94a3b8'
      }
    },
    y: {
      min: 0,
      max: 100,
      grid: { color: '#f1f5f9' },
      border: { display: false },
      ticks: {
        font: { size: 10 },
        color: '#94a3b8',
        callback: (value) => value + '%'
      }
    }
  },
  elements: {
    point: { radius: 0, hitRadius: 10, hoverRadius: 4 }
  }
}

const cpuChartData = computed(() => {
  const labels = historyData.value.map(d => new Date(d.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }))
  const data = historyData.value.map(d => d.cpu_usage.toFixed(1))
  
  return {
    labels,
    datasets: [{
      label: 'CPU Usage (%)',
      data,
      borderColor: '#3b82f6',
      backgroundColor: 'rgba(59, 130, 246, 0.1)',
      borderWidth: 2,
      fill: true,
      tension: 0.4
    }]
  }
})

const memChartData = computed(() => {
  const labels = historyData.value.map(d => new Date(d.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }))
  const data = historyData.value.map(d => {
    if (d.mem_total_bytes === 0) return 0
    return ((d.mem_used_bytes / d.mem_total_bytes) * 100).toFixed(1)
  })
  
  return {
    labels,
    datasets: [{
      label: 'Memory Usage (%)',
      data,
      borderColor: '#a855f7',
      backgroundColor: 'rgba(168, 85, 247, 0.1)',
      borderWidth: 2,
      fill: true,
      tension: 0.4
    }]
  }
})

const diskChartData = computed(() => {
  const labels = historyData.value.map(d => new Date(d.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }))
  const data = historyData.value.map(d => {
    if (!d.disk_total_bytes || d.disk_total_bytes === 0) return 0
    return ((d.disk_used_bytes / d.disk_total_bytes) * 100).toFixed(1)
  })
  
  return {
    labels,
    datasets: [{
      label: 'Disk Usage (%)',
      data,
      borderColor: '#f59e0b',
      backgroundColor: 'rgba(245, 158, 11, 0.1)',
      borderWidth: 2,
      fill: true,
      tension: 0.4
    }]
  }
})

onMounted(() => {
  connectWebSocket()
  fetchHistory()
})

onUnmounted(() => {
  if (ws) {
    const socket = ws
    ws = null // Null-kan referensi agar onclose tidak mencoba reconnect
    socket.close()
  }
})
</script>

<template>
  <div class="space-y-6">
    <div v-if="error" class="bg-red-50 text-red-600 p-4 rounded-md border border-red-200">
      Error connecting to backend: {{ error }}. Check Settings tab.
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      
      <!-- System Info (Left side, takes 2 cols on lg) -->
      <section class="space-y-6 lg:col-span-2">
        
        <!-- Live System Resources -->
        <div class="card">
          <h2 class="card-title"><Cpu class="w-5 h-5 text-brand-500" /> System Resources</h2>
          
          <div v-if="!sysInfo" class="flex items-center gap-2 text-slate-500 text-sm">
            <Loader2 class="w-4 h-4 animate-spin" /> Loading metrics...
          </div>

          <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
            <div class="p-4 bg-slate-50 rounded-lg border border-slate-100">
              <div class="text-sm font-medium text-slate-500 mb-1">Hostname &amp; OS</div>
              <div class="font-semibold text-slate-800 flex items-center gap-2">
                {{ sysInfo.hostname }}
                <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-700">
                  <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
                  {{ sysInfo.current_user }}
                </span>
              </div>
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
        </div>

        <!-- Historical Performance Charts -->
        <div class="card" v-if="fullHistoryData.length > 0">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-4">
            <div>
              <h2 class="card-title mb-1"><Activity class="w-5 h-5 text-brand-500" /> Historical Performance</h2>
              <p class="text-xs text-slate-500">Recorded every 5 minutes. Helps identify unexpected performance jumps.</p>
            </div>
            
            <!-- Time Range Filters -->
            <div class="flex flex-wrap items-center gap-1 bg-slate-100 p-1 rounded-lg">
              <button @click="setTimeRange('24h')" :class="historyTimeRange === '24h' ? 'bg-white shadow-sm text-slate-800' : 'text-slate-500 hover:text-slate-700'" class="px-3 py-1.5 rounded-md text-xs font-medium transition-all">24h</button>
              <button @click="setTimeRange('12h')" :class="historyTimeRange === '12h' ? 'bg-white shadow-sm text-slate-800' : 'text-slate-500 hover:text-slate-700'" class="px-3 py-1.5 rounded-md text-xs font-medium transition-all">12h</button>
              <button @click="setTimeRange('1h')" :class="historyTimeRange === '1h' ? 'bg-white shadow-sm text-slate-800' : 'text-slate-500 hover:text-slate-700'" class="px-3 py-1.5 rounded-md text-xs font-medium transition-all">1h</button>
              <button @click="setTimeRange('30m')" :class="historyTimeRange === '30m' ? 'bg-white shadow-sm text-slate-800' : 'text-slate-500 hover:text-slate-700'" class="px-3 py-1.5 rounded-md text-xs font-medium transition-all">30m</button>
              <button @click="setTimeRange('10m')" :class="historyTimeRange === '10m' ? 'bg-white shadow-sm text-slate-800' : 'text-slate-500 hover:text-slate-700'" class="px-3 py-1.5 rounded-md text-xs font-medium transition-all">10m</button>
              <button @click="setTimeRange('5m')" :class="historyTimeRange === '5m' ? 'bg-white shadow-sm text-slate-800' : 'text-slate-500 hover:text-slate-700'" class="px-3 py-1.5 rounded-md text-xs font-medium transition-all">5m</button>
            </div>
          </div>
          
          <div v-if="historyData.length === 0" class="flex flex-col items-center justify-center p-8 text-slate-400 bg-slate-50 rounded-lg border border-dashed border-slate-200">
            <Clock class="w-8 h-8 mb-2 opacity-50 text-slate-400" />
            <p class="text-sm">No data available for the selected time range ({{ historyTimeRange }}).</p>
          </div>
          <div v-else class="grid grid-cols-1 gap-6">
            <!-- CPU Chart -->
            <div class="h-48 relative w-full">
              <h3 class="text-sm font-semibold text-slate-700 mb-2">CPU Usage</h3>
              <div class="absolute inset-0 top-8">
                <Line :data="cpuChartData" :options="chartOptions" />
              </div>
            </div>
            
            <!-- Mem Chart -->
            <div class="h-48 relative w-full mt-4">
              <h3 class="text-sm font-semibold text-slate-700 mb-2">Memory Usage</h3>
              <div class="absolute inset-0 top-8">
                <Line :data="memChartData" :options="{ ...chartOptions, scales: { ...chartOptions.scales, y: { ...chartOptions.scales.y, max: 100 } } }" />
              </div>
            </div>
            
            <!-- Disk Chart -->
            <div class="h-48 relative w-full mt-4">
              <h3 class="text-sm font-semibold text-slate-700 mb-2">Disk Usage (Aggregated)</h3>
              <div class="absolute inset-0 top-8">
                <Line :data="diskChartData" :options="{ ...chartOptions, scales: { ...chartOptions.scales, y: { ...chartOptions.scales.y, max: 100 } } }" />
              </div>
            </div>
          </div>
        </div>
        <div class="card flex flex-col items-center justify-center p-8 text-slate-400" v-else>
          <Activity class="w-8 h-8 mb-2 opacity-50" />
          <p class="text-sm">Not enough historical data collected yet.</p>
          <p class="text-xs mt-1">Data is recorded every 5 minutes.</p>
        </div>

      </section>

      <!-- Task Manager (Right side, takes 1 col) -->
      <section class="card lg:col-span-1 flex flex-col h-[calc(100vh-12rem)] sticky top-6">
        
        <div class="flex items-center justify-between shrink-0 mb-3">
          <h2 class="card-title mb-0">Processes</h2>
          
          <div class="flex items-center gap-1 bg-slate-100 p-0.5 rounded-md">
            <button @click="processSortBy = 'cpu'" :class="processSortBy === 'cpu' ? 'bg-white shadow-sm text-slate-800' : 'text-slate-500 hover:text-slate-700'" class="px-2 py-1 rounded text-[10px] font-bold uppercase transition-all">CPU</button>
            <button @click="processSortBy = 'ram'" :class="processSortBy === 'ram' ? 'bg-white shadow-sm text-slate-800' : 'text-slate-500 hover:text-slate-700'" class="px-2 py-1 rounded text-[10px] font-bold uppercase transition-all">RAM</button>
          </div>
        </div>

        <div class="relative shrink-0 mb-3">
          <Search class="w-4 h-4 absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400" />
          <input v-model="processSearchQuery" type="text" placeholder="Search PID or Name..." class="w-full bg-slate-50 border border-slate-200 text-sm rounded-lg pl-9 pr-3 py-2 focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent transition-all">
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
            No processes found.
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
