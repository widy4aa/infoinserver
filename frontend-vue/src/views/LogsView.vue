<script setup>
import { ref, onMounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { AlertTriangle, AlertCircle, Info, Activity } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
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
      
      <div v-else-if="logs.length === 0" class="text-center p-8 border border-dashed border-slate-200 rounded-lg text-slate-500 bg-slate-50">
        <Activity class="w-8 h-8 mx-auto mb-2 text-slate-300" />
        <p>No activity logs or alerts found yet.</p>
        <p class="text-xs mt-1">Sistem akan otomatis mencatat di sini jika ada anomali (misal: CPU > 90%).</p>
      </div>

      <div v-else class="space-y-3">
        <div v-for="log in logs" :key="log.id" 
             class="p-4 rounded-lg border flex gap-4 items-start"
             :class="{
               'bg-red-50 border-red-200 text-red-800': log.level === 'CRITICAL',
               'bg-amber-50 border-amber-200 text-amber-800': log.level === 'WARNING',
               'bg-blue-50 border-blue-200 text-blue-800': log.level === 'INFO'
             }">
          
          <AlertCircle v-if="log.level === 'CRITICAL'" class="w-5 h-5 text-red-600 shrink-0 mt-0.5" />
          <AlertTriangle v-else-if="log.level === 'WARNING'" class="w-5 h-5 text-amber-500 shrink-0 mt-0.5" />
          <Info v-else class="w-5 h-5 text-blue-500 shrink-0 mt-0.5" />
          
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
