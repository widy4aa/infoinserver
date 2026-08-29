<script setup>
import { ref, onMounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useThemeStore } from '../stores/themeStore'
import { AlertTriangle, AlertCircle, Info, Activity } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { isDark } = useThemeStore()
const logs = ref([])
const isLoading = ref(true)

const fetchLogs = async () => {
  isLoading.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/logs/activity`)
    if (res.ok) {
      logs.value = await res.json()
    } else {
      const err = await res.text()
      console.error("Failed to fetch logs:", res.status, err)
    }
  } catch (e) {
    console.error("Failed to fetch logs", e)
  } finally {
    isLoading.value = false
  }
}

const formatDate = (isoString) => {
  return new Date(isoString).toLocaleString()
}

onMounted(() => {
  fetchLogs()
})
</script>

<template>
  <div class="space-y-6">
    <section class="card">
      <div class="flex items-center justify-between mb-4">
        <h2 class="card-title mb-0"><Activity class="w-5 h-5 text-brand-500" /> Activity Logs &amp; Alerts</h2>
        <button @click="fetchLogs" class="btn-outline btn-sm">Refresh</button>
      </div>
      <p class="text-sm text-slate-500 mb-6">
        Log ini dicatat secara otomatis oleh background scheduler ketika mendeteksi lonjakan pemakaian CPU, Memory, atau masalah Disk Space.
      </p>

      <div v-if="isLoading" class="flex justify-center p-8">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600"></div>
      </div>
      
      <div v-else-if="logs.length === 0" class="text-center p-8 border border-dashed rounded-lg" :class="isDark ? 'border-slate-700 text-slate-500 bg-slate-800/50' : 'border-slate-200 text-slate-500 bg-slate-50'">
        <Activity class="w-8 h-8 mx-auto mb-2 opacity-30" />
        <p>No activity logs or alerts found yet.</p>
        <p class="text-xs mt-1" :class="isDark ? 'text-slate-600' : 'text-slate-400'">Sistem akan otomatis mencatat di sini jika ada anomali (misal: CPU > 90%).</p>
      </div>

      <div v-else class="space-y-3">
        <div v-for="log in logs" :key="log.id" 
             class="p-4 rounded-lg border flex gap-4 items-start transition-colors"
             :class="
               log.level === 'CRITICAL' ? (isDark ? 'bg-red-900/10 border-red-800 text-red-400' : 'bg-red-50 border-red-200 text-red-800') :
               log.level === 'WARNING'  ? (isDark ? 'bg-amber-900/10 border-amber-800 text-amber-400' : 'bg-amber-50 border-amber-200 text-amber-800') :
               (isDark ? 'bg-blue-900/10 border-blue-800 text-blue-400' : 'bg-blue-50 border-blue-200 text-blue-800')
             ">
          
          <AlertCircle v-if="log.level === 'CRITICAL'" class="w-5 h-5 shrink-0 mt-0.5" :class="isDark ? 'text-red-500' : 'text-red-600'" />
          <AlertTriangle v-else-if="log.level === 'WARNING'" class="w-5 h-5 shrink-0 mt-0.5" :class="isDark ? 'text-amber-500' : 'text-amber-500'" />
          <Info v-else class="w-5 h-5 shrink-0 mt-0.5" :class="isDark ? 'text-blue-500' : 'text-blue-500'" />
          
          <div class="flex-1 min-w-0">
            <div class="flex items-center justify-between gap-4 mb-1">
              <h3 class="font-bold text-sm leading-tight">{{ log.action }}</h3>
              <span class="text-xs font-mono opacity-75 shrink-0">{{ formatDate(log.timestamp) }}</span>
            </div>
            <p class="text-sm opacity-90 leading-relaxed">{{ log.detail }}</p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
