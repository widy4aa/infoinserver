<script setup>
import { ref, onMounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import { Activity, Play, History, Download, Upload, Server } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast } = useToastStore()
const { isDark } = useThemeStore()

const history = ref([])
const isRunning = ref(false)
const isLoading = ref(true)

const fetchHistory = async () => {
  try {
    isLoading.value = true
    const res = await apiFetch(`${getActiveServerUrl()}/api/speedtest/history`)
    if (res.ok) {
      history.value = await res.json()
    }
  } catch (e) {
    showToast("Error", "Failed to fetch speedtest history", "error")
  } finally {
    isLoading.value = false
  }
}

const runTest = async () => {
  isRunning.value = true
  showToast("Info", "Starting Speedtest... This may take a minute.", "info")
  
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/speedtest/run`, {
      method: 'POST'
    })
    
    if (res.ok) {
      showToast("Success", "Speedtest completed", "success")
      await fetchHistory()
    } else {
      const err = await res.text()
      showToast("Error", `Speedtest failed: ${err}`, "error")
    }
  } catch (e) {
    showToast("Error", "Failed to run speedtest", "error")
  } finally {
    isRunning.value = false
  }
}

const formatMbps = (mbps) => {
  if (!mbps) return '0 Mbps'
  return parseFloat(mbps).toFixed(2) + ' Mbps'
}

const formatDate = (isoString) => {
  if (!isoString) return '-'
  return new Date(isoString).toLocaleString()
}

onMounted(() => {
  fetchHistory()
})
</script>

<template>
  <div class="space-y-6">
    <section class="card">
      <div class="flex items-center justify-between mb-6">
        <h2 class="card-title mb-0"><Activity class="w-5 h-5 text-brand-500" /> Network Speedtest</h2>
        <button @click="runTest" class="btn-primary" :disabled="isRunning">
          <Play v-if="!isRunning" class="w-4 h-4" />
          <Activity v-else class="w-4 h-4 animate-pulse" />
          {{ isRunning ? 'Testing...' : 'Run Speedtest' }}
        </button>
      </div>

      <div class="overflow-x-auto">
        <table class="w-full relative">
          <thead :class="isDark ? 'bg-slate-800/50' : 'bg-slate-50'">
            <tr>
              <th class="table-th">Date &amp; Time</th>
              <th class="table-th">Download</th>
              <th class="table-th">Upload</th>
              <th class="table-th">Ping</th>
              <th class="table-th">Server</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="isLoading">
              <td colspan="5" class="text-center p-8 text-slate-500">Loading history...</td>
            </tr>
            <tr v-else-if="history.length === 0">
              <td colspan="5" class="text-center p-8 text-slate-500">No speedtest history found. Run a test to begin.</td>
            </tr>
            <tr v-else v-for="item in history" :key="item.id" class="transition-colors" :class="isDark ? 'hover:bg-slate-800/50' : 'hover:bg-slate-50'">
              <td class="table-td text-xs text-slate-500">{{ formatDate(item.tested_at) }}</td>
              <td class="table-td font-semibold" :class="isDark ? 'text-green-400' : 'text-green-600'">
                <div class="flex items-center gap-1.5"><Download class="w-3.5 h-3.5" /> {{ formatMbps(item.download_mbps) }}</div>
              </td>
              <td class="table-td font-semibold" :class="isDark ? 'text-blue-400' : 'text-blue-600'">
                <div class="flex items-center gap-1.5"><Upload class="w-3.5 h-3.5" /> {{ formatMbps(item.upload_mbps) }}</div>
              </td>
              <td class="table-td font-medium" :class="isDark ? 'text-amber-400' : 'text-amber-600'">{{ item.ping_ms ? item.ping_ms.toFixed(1) + ' ms' : '-' }}</td>
              <td class="table-td text-xs">
                <div class="flex items-center gap-1.5 text-slate-500 dark:text-slate-400">
                  <Server class="w-3.5 h-3.5" /> 
                  <span class="truncate max-w-[150px]" :title="item.server_name || 'Unknown'">{{ item.server_name || 'Unknown' }}</span>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>
