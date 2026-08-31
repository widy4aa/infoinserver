<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useThemeStore } from '../stores/themeStore'
import { Cpu, Loader2, Activity, Clock } from 'lucide-vue-next'
import { getDistroIcon } from '../utils/distro.js'

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
const { getActiveServerUrl, getToken, activeServerId, setServerOsName } = useServerStore()
const { isDark } = useThemeStore()

const sysInfo = ref(null)
const error = ref(null)
let ws = null
let osNameSaved = false // Flag agar tidak berulang kali simpan

// History Data
const historyData = ref([])
const fullHistoryData = ref([])
const historyTimeRange = ref('24h')

const filterHistoryByTime = (range) => {
  if (!fullHistoryData.value || fullHistoryData.value.length === 0) return []
  
  const now = new Date()
  let limitTime = new Date()
  
  switch(range) {
    case '1h': limitTime.setHours(now.getHours() - 1); break;
    case '3h': limitTime.setHours(now.getHours() - 3); break;
    case '6h': limitTime.setHours(now.getHours() - 6); break;
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
        error.value = null
        // Simpan os_name ke localStorage sekali saja (Opsi C - Hybrid)
        if (!osNameSaved && data.system?.os_name) {
          setServerOsName(activeServerId.value, data.system.os_name)
          osNameSaved = true
        }
      }
    } catch (e) {
      console.error('Failed to parse WS data', e)
    }
  }

  ws.onerror = () => {
    error.value = "WebSocket connection error. Check backend or auth."
  }

  ws.onclose = () => {
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
      historyData.value = filterHistoryByTime(historyTimeRange.value)
    }
  } catch (e) {
    console.error("Failed to fetch metrics history", e)
  }
}

const formatUptime = (seconds) => {
  if (!seconds) return '0m'
  const d = Math.floor(seconds / 86400)
  const h = Math.floor((seconds % 86400) / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return `${d}d ${h}h ${m}m`
}

// ── Dark-aware Chart Configurations ──
const chartColors = computed(() => ({
  // Grid/tick colors
  grid: isDark.value ? '#1e293b' : '#f1f5f9',
  tick: isDark.value ? '#475569' : '#94a3b8',
  tooltipBg: isDark.value ? 'rgba(15, 23, 42, 0.95)' : 'rgba(15, 23, 42, 0.9)',
  
  // Dataset colors (same, work on both backgrounds)
  cpu: { border: '#3b82f6', fill: 'rgba(59, 130, 246, 0.15)' },
  mem: { border: '#a855f7', fill: 'rgba(168, 85, 247, 0.15)' },
  disk: { border: '#f59e0b', fill: 'rgba(245, 158, 11, 0.15)' },
  netRx: '#10b981',
  netTx: '#ef4444',
}))

const chartOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: false,
  interaction: {
    mode: 'index',
    intersect: false,
  },
  plugins: {
    legend: { 
      display: false,
      labels: {
        color: isDark.value ? '#e2e8f0' : '#1e293b',
        font: { size: 10 }
      }
    },
    tooltip: {
      backgroundColor: chartColors.value.tooltipBg,
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
        color: chartColors.value.tick
      }
    },
    y: {
      min: 0,
      max: 100,
      grid: { color: chartColors.value.grid },
      border: { display: false },
      ticks: {
        font: { size: 10 },
        color: chartColors.value.tick,
        callback: (value) => value + '%'
      }
    }
  },
  elements: {
    point: { radius: 0, hitRadius: 10, hoverRadius: 4 }
  }
}))

const cpuChartData = computed(() => {
  const labels = historyData.value.map(d => new Date(d.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }))
  const data = historyData.value.map(d => d.cpu_usage.toFixed(1))
  
  return {
    labels,
    datasets: [{
      label: 'CPU Usage (%)',
      data,
      borderColor: chartColors.value.cpu.border,
      backgroundColor: chartColors.value.cpu.fill,
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
      borderColor: chartColors.value.mem.border,
      backgroundColor: chartColors.value.mem.fill,
      borderWidth: 2,
      fill: true,
      tension: 0.4
    }]
  }
})

const diskChartData = computed(() => {
  const labels = historyData.value.map(d => new Date(d.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }))
  const data = historyData.value.map(d => {
    if (!d || !d.disk_total_bytes || d.disk_total_bytes === 0) return 0
    return ((d.disk_used_bytes / d.disk_total_bytes) * 100).toFixed(1)
  })
  
  return {
    labels,
    datasets: [{
      label: 'Disk Usage (%)',
      data,
      borderColor: chartColors.value.disk.border,
      backgroundColor: chartColors.value.disk.fill,
      borderWidth: 2,
      fill: true,
      tension: 0.4
    }]
  }
})

const netChartData = computed(() => {
  if (historyData.value.length < 2) return { labels: [], datasets: [] }
  
  const labels = []
  const rxData = []
  const txData = []

  for (let i = 1; i < historyData.value.length; i++) {
    const prev = historyData.value[i - 1]
    const curr = historyData.value[i]
    
    const prevRx = prev.net_rx_bytes || 0
    const prevTx = prev.net_tx_bytes || 0
    const currRx = curr.net_rx_bytes || 0
    const currTx = curr.net_tx_bytes || 0

    const timeDiffSec = (new Date(curr.timestamp) - new Date(prev.timestamp)) / 1000
    
    let rxMbps = 0
    let txMbps = 0
    
    if (timeDiffSec > 0 && currRx >= prevRx && currTx >= prevTx) {
      const rxBytesPerSec = (currRx - prevRx) / timeDiffSec
      const txBytesPerSec = (currTx - prevTx) / timeDiffSec
      rxMbps = (rxBytesPerSec * 8) / 1000000
      txMbps = (txBytesPerSec * 8) / 1000000
    }

    labels.push(new Date(curr.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }))
    rxData.push(rxMbps.toFixed(2))
    txData.push(txMbps.toFixed(2))
  }
  
  return {
    labels,
    datasets: [
      {
        label: 'Download (Mbps)',
        data: rxData,
        borderColor: chartColors.value.netRx,
        backgroundColor: 'transparent',
        borderWidth: 2,
        tension: 0.4
      },
      {
        label: 'Upload (Mbps)',
        data: txData,
        borderColor: chartColors.value.netTx,
        backgroundColor: 'transparent',
        borderWidth: 2,
        tension: 0.4
      }
    ]
  }
})

onMounted(() => {
  connectWebSocket()
  fetchHistory()
})

onUnmounted(() => {
  if (ws) {
    const socket = ws
    ws = null
    socket.close()
  }
})
</script>

<template>
  <div class="space-y-6">
    <div v-if="error" class="bg-red-50 text-red-600 p-4 rounded-md border border-red-200 dark:bg-red-900/30 dark:text-red-300 dark:border-red-800">
      Error connecting to backend: {{ error }}. Check Settings tab.
    </div>

    <div class="flex flex-col gap-6">
      <section class="flex flex-col gap-6">
        <!-- Live System Resources -->
        <div class="card">
          <h2 class="card-title"><Cpu class="w-5 h-5 text-brand-500" /> System Resources</h2>
          
          <div v-if="!sysInfo" class="flex items-center gap-2 text-slate-500 dark:text-slate-400 text-sm">
            <Loader2 class="w-4 h-4 animate-spin" /> Loading metrics...
          </div>

          <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4 items-stretch">
            <div class="p-4 bg-slate-50 rounded-lg border border-slate-100 flex flex-col justify-center dark:bg-slate-800/50 dark:border-slate-700">
              <div class="text-sm font-medium text-slate-500 dark:text-slate-400 mb-1">Hostname & OS</div>
              <div class="font-semibold text-slate-800 dark:text-slate-100 flex items-center gap-2">
                <!-- Distro Icon -->
                <img v-if="getDistroIcon(sysInfo.os_name)" :src="getDistroIcon(sysInfo.os_name)" :alt="sysInfo.os_name" class="w-5 h-5 object-contain shrink-0" />
                {{ sysInfo.hostname }}
                <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300">
                  <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
                  {{ sysInfo.current_user }}
                </span>
              </div>
              <div class="text-xs text-slate-500 dark:text-slate-400 mt-1">{{ sysInfo.os_name }} • {{ sysInfo.kernel_version }}</div>
            </div>
            
            <div class="p-4 bg-green-50 rounded-lg border border-green-100 flex flex-col justify-center dark:bg-green-900/20 dark:border-green-800">
              <div class="text-sm font-medium text-green-600 dark:text-green-400 mb-1">Uptime</div>
              <div class="font-bold text-green-700 dark:text-green-300 text-lg">{{ formatUptime(sysInfo.uptime) }}</div>
            </div>
            
            <div class="p-4 bg-brand-50 rounded-lg border border-brand-100 md:col-span-2 dark:bg-blue-900/20 dark:border-blue-800">
              <div class="flex flex-col gap-2">
                <div>
                  <div class="text-sm font-medium text-brand-600 dark:text-brand-400">CPU Usage ({{ sysInfo.cpu_cores }} Cores)</div>
                  <div class="text-xs text-brand-500/70 dark:text-brand-400/60 font-mono truncate mt-0.5 w-full" :title="sysInfo.cpu_model">
                    {{ sysInfo.cpu_model }}
                  </div>
                </div>
                <div class="flex items-center gap-3">
                  <div class="text-2xl font-bold text-brand-700 dark:text-brand-300 w-20">{{ sysInfo.global_cpu_usage?.toFixed(1) }}%</div>
                  <div class="flex-1 bg-brand-200 rounded-full h-2.5 overflow-hidden dark:bg-brand-800">
                    <div class="bg-brand-500 h-2.5 rounded-full transition-all duration-300" :style="`width: ${Math.min(Math.max(sysInfo.global_cpu_usage||0, 0), 100)}%`"></div>
                  </div>
                </div>
              </div>
            </div>
            
            <div class="p-4 bg-purple-50 rounded-lg border border-purple-100 md:col-span-2 dark:bg-purple-900/20 dark:border-purple-800">
              <div class="text-sm font-medium text-purple-600 dark:text-purple-400 mb-2">Memory (RAM)</div>
              <div class="flex items-center gap-3">
                <div class="font-bold text-purple-700 dark:text-purple-300 w-32 whitespace-nowrap text-sm">
                  {{ (sysInfo.used_memory / 1024 / 1024 / 1024).toFixed(2) }} GB / {{ (sysInfo.total_memory / 1024 / 1024 / 1024).toFixed(2) }} GB
                </div>
                <div class="flex-1 bg-purple-200 rounded-full h-2.5 overflow-hidden dark:bg-purple-800">
                  <div class="bg-purple-500 h-2.5 rounded-full transition-all duration-300" :style="`width: ${Math.min(sysInfo.used_memory/sysInfo.total_memory*100, 100)}%`"></div>
                </div>
              </div>
            </div>
            
            <div class="p-4 bg-amber-50 rounded-lg border border-amber-100 md:col-span-2 dark:bg-amber-900/20 dark:border-amber-800">
              <div class="text-sm font-medium text-amber-600 dark:text-amber-400 mb-2">Storage (Disks)</div>
              <div class="space-y-3 max-h-48 overflow-y-auto pr-1">
                <div v-for="disk in sysInfo.disks" :key="disk.mount_point" class="flex flex-col">
                  <div class="flex justify-between items-center text-xs mb-1" v-if="disk.total_space > 0">
                    <span class="font-medium text-amber-900 dark:text-amber-100 truncate flex-1 pr-2" :title="disk.mount_point">
                      {{ disk.mount_point }} <span class="text-amber-600 dark:text-amber-400 font-normal">({{ disk.name }})</span>
                    </span>
                    <span class="text-amber-700 dark:text-amber-300 whitespace-nowrap">
                      {{ ((disk.total_space - disk.available_space)/1073741824).toFixed(1) }} GB / {{ (disk.total_space/1073741824).toFixed(1) }} GB
                    </span>
                  </div>
                  <div class="w-full bg-amber-200 rounded-full h-2 overflow-hidden dark:bg-amber-800" v-if="disk.total_space > 0">
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
              <p class="text-xs text-slate-500 dark:text-slate-400">Recorded every 5 minutes. Helps identify unexpected performance jumps.</p>
            </div>
            
            <!-- Time Range Filters -->
            <div class="flex flex-wrap items-center gap-1 bg-slate-100 p-1 rounded-lg dark:bg-slate-800">
              <button @click="setTimeRange('24h')" :class="historyTimeRange === '24h' ? 'bg-white shadow-sm text-slate-800 dark:bg-slate-700 dark:text-slate-100' : 'text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200'" class="px-3 py-1.5 rounded-md text-xs font-medium transition-all">24h</button>
              <button @click="setTimeRange('12h')" :class="historyTimeRange === '12h' ? 'bg-white shadow-sm text-slate-800 dark:bg-slate-700 dark:text-slate-100' : 'text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200'" class="px-3 py-1.5 rounded-md text-xs font-medium transition-all">12h</button>
              <button @click="setTimeRange('6h')" :class="historyTimeRange === '6h' ? 'bg-white shadow-sm text-slate-800 dark:bg-slate-700 dark:text-slate-100' : 'text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200'" class="px-3 py-1.5 rounded-md text-xs font-medium transition-all">6h</button>
              <button @click="setTimeRange('3h')" :class="historyTimeRange === '3h' ? 'bg-white shadow-sm text-slate-800 dark:bg-slate-700 dark:text-slate-100' : 'text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200'" class="px-3 py-1.5 rounded-md text-xs font-medium transition-all">3h</button>
              <button @click="setTimeRange('1h')" :class="historyTimeRange === '1h' ? 'bg-white shadow-sm text-slate-800 dark:bg-slate-700 dark:text-slate-100' : 'text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200'" class="px-3 py-1.5 rounded-md text-xs font-medium transition-all">1h</button>
            </div>
          </div>
          
          <div v-if="historyData.length < 2" class="flex flex-col items-center justify-center p-8 text-slate-400 dark:text-slate-500 bg-slate-50 rounded-lg border border-dashed border-slate-200 dark:bg-slate-800/50 dark:border-slate-700">
            <Clock class="w-8 h-8 mb-2 opacity-50" />
            <p class="text-sm">Not enough data available for the selected time range ({{ historyTimeRange }}).</p>
            <p class="text-xs mt-1">Chart requires at least 2 data points (10 minutes of recording).</p>
          </div>
          <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <!-- CPU Chart -->
            <div class="h-48 relative w-full">
              <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-2">CPU Usage</h3>
              <div class="absolute inset-0 top-8">
                <Line :data="cpuChartData" :options="chartOptions" />
              </div>
            </div>
            
            <!-- Mem Chart -->
            <div class="h-48 relative w-full">
              <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-2">Memory Usage</h3>
              <div class="absolute inset-0 top-8">
                <Line :data="memChartData" :options="{ ...chartOptions, scales: { ...chartOptions.scales, y: { ...chartOptions.scales.y, max: 100 } } }" />
              </div>
            </div>
            
            <!-- Disk Chart -->
            <div class="h-48 relative w-full">
              <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-2">Disk Usage</h3>
              <div class="absolute inset-0 top-8">
                <Line :data="diskChartData" :options="{ ...chartOptions, scales: { ...chartOptions.scales, y: { ...chartOptions.scales.y, max: 100 } } }" />
              </div>
            </div>
            
            <!-- Network Bandwidth Chart -->
            <div class="h-48 relative w-full">
              <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-2">Network (Mbps)</h3>
              <div class="absolute inset-0 top-8">
                <Line :data="netChartData" :options="{ ...chartOptions, plugins: { legend: { display: true, position: 'top', labels: { boxWidth: 10, usePointStyle: true, font: {size: 10}, color: isDark ? '#e2e8f0' : '#1e293b' } }, tooltip: chartOptions.plugins.tooltip }, scales: { ...chartOptions.scales, y: { ...chartOptions.scales.y, max: undefined, ticks: { ...chartOptions.scales.y.ticks, callback: (v) => v + ' Mbps' } } } }" />
              </div>
            </div>
          </div>
        </div>
        <div class="card flex flex-col items-center justify-center p-8 text-slate-400 dark:text-slate-500" v-else>
          <Activity class="w-8 h-8 mb-2 opacity-50" />
          <p class="text-sm">Not enough historical data collected yet.</p>
          <p class="text-xs mt-1">Data is recorded every 5 minutes.</p>
        </div>

      </section>

    </div>
  </div>
</template>