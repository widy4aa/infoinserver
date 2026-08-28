<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { Cloud, CheckCircle2, XCircle, Play, Square, Loader2, DownloadCloud } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showConfirm, showToast } = useToastStore()

const status = ref(null)
const isLoading = ref(true)
const actionMsg = ref('')

const quickPort = ref('8080')
const managedToken = ref('')

let pollInterval = null

const fetchStatus = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/status`)
    if (res.ok) {
      status.value = await res.json()
    }
  } catch (e) {
    console.error(e)
  } finally {
    isLoading.value = false
  }
}

const handleAction = async (endpoint, payload = null) => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/${endpoint}`, {
      method: 'POST',
      headers: payload ? { 'Content-Type': 'application/json' } : {},
      body: payload ? JSON.stringify(payload) : null
    })
    
    const data = await res.json()
    
    if (res.ok) {
      showToast("Success", data.message, "success")
      fetchStatus()
    } else {
      showToast("Error", `Error: ${data}`, "error")
    }
  } catch (e) {
    showToast("Error", `Failed: ${e.message}`, "error")
  }
}

const confirmStopTunnel = () => {
  showConfirm("Konfirmasi", "Hentikan tunnel cloudflared yang sedang berjalan?", () => handleAction('stop'))
}

onMounted(() => {
  fetchStatus()
  pollInterval = setInterval(fetchStatus, 3000) // Poll for active URLs frequently
})

onUnmounted(() => {
  clearInterval(pollInterval)
})
</script>

<template>
  <section class="card">
    <h2 class="card-title"><Cloud class="w-5 h-5 text-brand-500" /> Cloudflare Tunnels</h2>
    
    <div v-if="isLoading" class="text-sm text-slate-500 flex items-center gap-2">
      <Loader2 class="w-4 h-4 animate-spin" /> Loading status...
    </div>

    <div v-else-old class="space-y-6">
      
      <!-- Status Badge -->
      <div class="flex items-center gap-4 p-4 bg-slate-50 rounded-lg border border-slate-200">
        <div class="flex-1">
          <div class="text-sm font-semibold text-slate-700">Service Status</div>
          <div class="flex items-center gap-2 mt-1">
            <span v-if="status.installed" class="flex items-center gap-1 text-xs font-medium text-green-600"><CheckCircle2 class="w-3 h-3" /> Installed</span>
            <span v-else class="flex items-center gap-1 text-xs font-medium text-red-500"><XCircle class="w-3 h-3" /> Not Installed</span>
            
            <span class="text-slate-300">|</span>
            
            <span v-if="status.running" class="flex items-center gap-1 text-xs font-medium text-green-600"><CheckCircle2 class="w-3 h-3" /> Running</span>
            <span v-else class="flex items-center gap-1 text-xs font-medium text-slate-500"><Square class="w-3 h-3" /> Stopped</span>
          </div>
        </div>
        
        <button v-if="!status.installed" @click="handleAction('install')" class="btn-primary">
          <DownloadCloud class="w-4 h-4" /> Install Cloudflared
        </button>
        <button v-if="status.running" @click="confirmStopTunnel" class="btn-destructive">
          <Square class="w-4 h-4" /> Stop Tunnel
        </button>
      </div>

      <!-- Controls (Only show if installed) -->
      <div v-if="status.installed" class="grid grid-cols-1 lg:grid-cols-2 gap-4">
        
        <!-- Quick Tunnel -->
        <div class="p-4 border border-slate-200 rounded-lg">
          <h3 class="font-semibold text-sm mb-1 text-slate-800">Quick Tunnel (TryCloudflare)</h3>
          <p class="text-xs text-slate-500 mb-4">Expose a local port temporarily. URL changes on restart.</p>
          
          <div class="flex gap-2 mb-4">
            <input v-model="quickPort" type="text" placeholder="Port (e.g. 8080)" class="input-field max-w-[100px]">
            <button @click="handleAction('quick', { port: quickPort })" class="btn-primary whitespace-nowrap">
              <Play class="w-4 h-4" /> Start Quick
            </button>
          </div>

          <div v-if="status.active_tunnels && status.active_tunnels.length > 0" class="mt-2 bg-brand-50 p-2 rounded border border-brand-100">
            <div class="text-[10px] font-bold text-brand-600 mb-1 uppercase tracking-wider">Active URL:</div>
            <a v-for="url in status.active_tunnels" :key="url" :href="url" target="_blank" class="block font-mono text-sm text-brand-700 hover:underline break-all">
              {{ url }}
            </a>
          </div>
        </div>

        <!-- Managed Tunnel -->
        <div class="p-4 border border-slate-200 rounded-lg">
          <h3 class="font-semibold text-sm mb-1 text-slate-800">Managed Tunnel (Zero Trust)</h3>
          <p class="text-xs text-slate-500 mb-4">Run as a permanent service using your Cloudflare account.</p>
          
          <div class="flex flex-col gap-2">
            <input v-model="managedToken" type="password" placeholder="eyJhIjoi..." class="input-field font-mono text-xs">
            <button @click="handleAction('managed', { token: managedToken })" class="btn-outline text-brand-600">
              <CheckCircle2 class="w-4 h-4" /> Install Service
            </button>
          </div>
        </div>
      </div>

    </div>
  </section>
</template>