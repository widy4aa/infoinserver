<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useApi } from '../composables/useApi'
import { isTokenExpired } from '../composables/useApi'
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
      // Cek apakah token expired sebelum reconnect
      const currentToken = getToken(activeServerId.value)
      if (isTokenExpired(currentToken, 0)) {
        // Token sudah expire — minta login ulang via modal, jangan reconnect
        window.dispatchEvent(new CustomEvent('auth:expired', {
          detail: { serverId: activeServerId.value }
        }))
        return
      }
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
  cpu: { border: '#06b6d4', fill: 'rgba(6, 182, 212, 0.12)' },
  mem: { border: '#a855f7', fill: 'rgba(168, 85, 247, 0.12)' },
  disk: { border: '#f59e0b', fill: 'rgba(245, 158, 11, 0.12)' },
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

          <div v-else class="space-y-6 mt-4">

            <!-- Row 1: Hostname + Uptime stat cards -->
            <div class="grid grid-cols-2 gap-3">
              <!-- Hostname card -->
              <div class="rounded-xl p-4 border border-slate-100 dark:border-slate-700 bg-slate-50/80 dark:bg-slate-800/40 space-y-1">
                <div class="text-[10px] font-semibold uppercase tracking-[0.08em] text-slate-400 dark:text-slate-500">Hostname</div>
                <div class="flex items-center gap-2 min-w-0">
                  <img v-if="getDistroIcon(sysInfo.os_name)" :src="getDistroIcon(sysInfo.os_name)" :alt="sysInfo.os_name" class="w-4 h-4 object-contain shrink-0" />
                  <span class="font-bold text-sm text-slate-800 dark:text-slate-100 truncate">{{ sysInfo.hostname }}</span>
                </div>
                <div class="text-[10px] text-slate-400 dark:text-slate-500 font-mono truncate">{{ sysInfo.os_name }}</div>
                <div class="text-[10px] text-slate-400 dark:text-slate-600 font-mono truncate">{{ sysInfo.kernel_version }}</div>
              </div>

              <!-- Uptime card -->
              <div class="rounded-xl p-4 border border-green-100 dark:border-green-900/40 bg-green-50/60 dark:bg-green-900/10 space-y-1">
                <div class="text-[10px] font-semibold uppercase tracking-[0.08em] text-green-500 dark:text-green-600">Uptime</div>
                <div class="font-bold text-lg text-green-700 dark:text-green-300 leading-tight">{{ formatUptime(sysInfo.uptime) }}</div>
                <div class="flex items-center gap-1.5 mt-1">
                  <span class="text-[10px] font-semibold uppercase tracking-[0.06em]"
                        :class="isDark ? 'text-cyan-400' : 'text-cyan-600'">
                    {{ sysInfo.current_user }}
                  </span>
                </div>
              </div>
            </div>

            <!-- Row 2: CPU + RAM + Swap + Temp — 4 donut gauges -->
            <div class="grid grid-cols-2 md:grid-cols-4 gap-3">

              <!-- CPU gauge -->
              <div class="rounded-xl p-4 border border-cyan-100 dark:border-cyan-900/30 bg-cyan-50/40 dark:bg-cyan-900/8 flex flex-col items-center gap-3">
                <div class="text-[10px] font-semibold uppercase tracking-[0.08em] text-brand-500 self-start">CPU</div>
                <div class="relative w-20 h-20">
                  <svg viewBox="0 0 36 36" class="w-20 h-20 -rotate-90">
                    <circle cx="18" cy="18" r="15.9" fill="none"
                      :stroke="isDark ? 'rgba(6,182,212,0.15)' : 'rgba(6,182,212,0.12)'"
                      stroke-width="3.2" />
                    <circle cx="18" cy="18" r="15.9" fill="none"
                      stroke="#06b6d4" stroke-width="3.2" stroke-linecap="round"
                      :stroke-dasharray="`${(sysInfo.global_cpu_usage||0) * 0.999} 100`"
                      style="transition: stroke-dasharray 0.4s ease" />
                  </svg>
                  <div class="absolute inset-0 flex flex-col items-center justify-center">
                    <span class="text-lg font-bold text-brand-600 dark:text-brand-300 leading-none">{{ (sysInfo.global_cpu_usage||0).toFixed(0) }}</span>
                    <span class="text-[9px] text-slate-400 font-mono">%</span>
                  </div>
                </div>
                <div class="text-center space-y-0.5 w-full">
                  <div class="text-[10px] text-slate-500 dark:text-slate-400 font-mono truncate" :title="sysInfo.cpu_model">{{ sysInfo.cpu_model }}</div>
                  <div class="text-[10px] text-slate-400 dark:text-slate-500">{{ sysInfo.cpu_cores }} cores</div>
                </div>
              </div>

              <!-- RAM gauge -->
              <div class="rounded-xl p-4 border border-purple-100 dark:border-purple-900/30 bg-purple-50/40 dark:bg-purple-900/8 flex flex-col items-center gap-3">
                <div class="text-[10px] font-semibold uppercase tracking-[0.08em] text-purple-500 self-start">Memory</div>
                <div class="relative w-20 h-20">
                  <svg viewBox="0 0 36 36" class="w-20 h-20 -rotate-90">
                    <circle cx="18" cy="18" r="15.9" fill="none"
                      :stroke="isDark ? 'rgba(168,85,247,0.15)' : 'rgba(168,85,247,0.12)'"
                      stroke-width="3.2" />
                    <circle cx="18" cy="18" r="15.9" fill="none"
                      stroke="#a855f7" stroke-width="3.2" stroke-linecap="round"
                      :stroke-dasharray="`${Math.min(sysInfo.used_memory/sysInfo.total_memory*100,100)*0.999} 100`"
                      style="transition: stroke-dasharray 0.4s ease" />
                  </svg>
                  <div class="absolute inset-0 flex flex-col items-center justify-center">
                    <span class="text-lg font-bold text-purple-600 dark:text-purple-300 leading-none">{{ (sysInfo.used_memory/sysInfo.total_memory*100).toFixed(0) }}</span>
                    <span class="text-[9px] text-slate-400 font-mono">%</span>
                  </div>
                </div>
                <div class="text-center space-y-0.5">
                  <div class="text-xs font-semibold text-purple-700 dark:text-purple-300">{{ (sysInfo.used_memory/1073741824).toFixed(1) }} GB</div>
                  <div class="text-[10px] text-slate-400 dark:text-slate-500">of {{ (sysInfo.total_memory/1073741824).toFixed(1) }} GB</div>
                </div>
              </div>

              <!-- Swap gauge -->
              <div class="rounded-xl p-4 border flex flex-col items-center gap-3"
                   :class="sysInfo.total_swap > 0
                     ? 'border-indigo-100 dark:border-indigo-900/30 bg-indigo-50/40 dark:bg-indigo-900/8'
                     : 'border-slate-100 dark:border-slate-700 bg-slate-50/40 dark:bg-slate-800/30 opacity-50'">
                <div class="text-[10px] font-semibold uppercase tracking-[0.08em] self-start"
                     :class="sysInfo.total_swap > 0 ? 'text-indigo-500' : 'text-slate-400'">Swap</div>
                <div class="relative w-20 h-20">
                  <svg viewBox="0 0 36 36" class="w-20 h-20 -rotate-90">
                    <circle cx="18" cy="18" r="15.9" fill="none"
                      :stroke="isDark ? 'rgba(99,102,241,0.15)' : 'rgba(99,102,241,0.12)'"
                      stroke-width="3.2" />
                    <circle v-if="sysInfo.total_swap > 0" cx="18" cy="18" r="15.9" fill="none"
                      stroke="#6366f1" stroke-width="3.2" stroke-linecap="round"
                      :stroke-dasharray="`${Math.min(sysInfo.used_swap/sysInfo.total_swap*100,100)*0.999} 100`"
                      style="transition: stroke-dasharray 0.4s ease" />
                  </svg>
                  <div class="absolute inset-0 flex flex-col items-center justify-center">
                    <span class="text-lg font-bold leading-none"
                          :class="sysInfo.total_swap > 0 ? 'text-indigo-600 dark:text-indigo-300' : 'text-slate-400'">
                      {{ sysInfo.total_swap > 0 ? (sysInfo.used_swap/sysInfo.total_swap*100).toFixed(0) : '—' }}
                    </span>
                    <span class="text-[9px] text-slate-400 font-mono">{{ sysInfo.total_swap > 0 ? '%' : '' }}</span>
                  </div>
                </div>
                <div class="text-center space-y-0.5">
                  <div v-if="sysInfo.total_swap > 0" class="text-xs font-semibold text-indigo-700 dark:text-indigo-300">{{ (sysInfo.used_swap/1073741824).toFixed(1) }} GB</div>
                  <div v-else class="text-xs text-slate-400">No swap</div>
                  <div v-if="sysInfo.total_swap > 0" class="text-[10px] text-slate-400 dark:text-slate-500">of {{ (sysInfo.total_swap/1073741824).toFixed(1) }} GB</div>
                </div>
              </div>

              <!-- CPU Temp gauge -->
              <div class="rounded-xl p-4 border flex flex-col items-center gap-3"
                   :class="sysInfo.cpu_temp != null
                     ? 'border-rose-100 dark:border-rose-900/30 bg-rose-50/40 dark:bg-rose-900/8'
                     : 'border-slate-100 dark:border-slate-700 bg-slate-50/40 dark:bg-slate-800/30 opacity-50'">
                <div class="text-[10px] font-semibold uppercase tracking-[0.08em] self-start"
                     :class="sysInfo.cpu_temp != null ? 'text-rose-500' : 'text-slate-400'">Temp</div>
                <div class="relative w-20 h-20">
                  <svg viewBox="0 0 36 36" class="w-20 h-20 -rotate-90">
                    <circle cx="18" cy="18" r="15.9" fill="none"
                      :stroke="isDark ? 'rgba(244,63,94,0.15)' : 'rgba(244,63,94,0.12)'"
                      stroke-width="3.2" />
                    <circle v-if="sysInfo.cpu_temp != null" cx="18" cy="18" r="15.9" fill="none"
                      :stroke="sysInfo.cpu_temp > 85 ? '#ef4444' : sysInfo.cpu_temp > 70 ? '#f59e0b' : '#f43f5e'"
                      stroke-width="3.2" stroke-linecap="round"
                      :stroke-dasharray="`${Math.min(sysInfo.cpu_temp / 100 * 100, 100) * 0.999} 100`"
                      style="transition: stroke-dasharray 0.4s ease" />
                  </svg>
                  <div class="absolute inset-0 flex flex-col items-center justify-center">
                    <span class="text-lg font-bold leading-none"
                          :class="sysInfo.cpu_temp != null
                            ? (sysInfo.cpu_temp > 85 ? 'text-red-600 dark:text-red-400' : sysInfo.cpu_temp > 70 ? 'text-amber-500' : 'text-rose-600 dark:text-rose-300')
                            : 'text-slate-400'">
                      {{ sysInfo.cpu_temp != null ? sysInfo.cpu_temp.toFixed(0) : '—' }}
                    </span>
                    <span class="text-[9px] text-slate-400 font-mono">°C</span>
                  </div>
                </div>
                <div class="text-center">
                  <div v-if="sysInfo.cpu_temp != null" class="text-[10px] font-semibold"
                       :class="sysInfo.cpu_temp > 85 ? 'text-red-500' : sysInfo.cpu_temp > 70 ? 'text-amber-500' : 'text-rose-400'">
                    {{ sysInfo.cpu_temp > 85 ? 'Critical' : sysInfo.cpu_temp > 70 ? 'Warm' : 'Normal' }}
                  </div>
                  <div v-else class="text-[10px] text-slate-400">Not available</div>
                </div>
              </div>

            </div>

            <!-- Row 3: Disk — compact bars -->
            <div class="rounded-xl p-4 border border-amber-100 dark:border-amber-900/30 bg-amber-50/40 dark:bg-amber-900/8">
              <div class="text-[10px] font-semibold uppercase tracking-[0.08em] text-amber-500 mb-3">Storage</div>
              <div class="space-y-2.5 max-h-40 overflow-y-auto pr-1">
                <div v-for="disk in sysInfo.disks" :key="disk.mount_point">
                  <div v-if="disk.total_space > 0" class="space-y-1">
                    <div class="flex justify-between items-center">
                      <span class="text-xs font-medium truncate flex-1 pr-2"
                            :class="isDark ? 'text-amber-200' : 'text-amber-900'"
                            :title="disk.mount_point">
                        {{ disk.mount_point }}
                        <span class="font-normal" :class="isDark ? 'text-amber-500' : 'text-amber-500'"> {{ disk.name }}</span>
                      </span>
                      <span class="text-[10px] font-mono whitespace-nowrap" :class="isDark ? 'text-amber-400' : 'text-amber-600'">
                        {{ ((disk.total_space-disk.available_space)/1073741824).toFixed(1) }} / {{ (disk.total_space/1073741824).toFixed(1) }} GB
                      </span>
                    </div>
                    <!-- Segmented bar -->
                    <div class="w-full h-1.5 rounded-full overflow-hidden"
                         :class="isDark ? 'bg-amber-900/40' : 'bg-amber-200/60'">
                      <div class="h-full rounded-full transition-all duration-300"
                           :class="((disk.total_space-disk.available_space)/disk.total_space*100) > 90
                             ? 'bg-red-500'
                             : ((disk.total_space-disk.available_space)/disk.total_space*100) > 70
                               ? 'bg-amber-500'
                               : 'bg-amber-400'"
                           :style="`width: ${Math.min(((disk.total_space-disk.available_space)/disk.total_space)*100,100)}%`" />
                    </div>
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