<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { Cloud, CheckCircle2, XCircle, Loader2, DownloadCloud, RefreshCw, ShieldCheck, AlertTriangle } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast } = useToastStore()

const status = ref(null)
const isLoading = ref(true)
const isInstalling = ref(false)

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

const installCloudflared = async () => {
  isInstalling.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/install`, { method: 'POST' })
    const data = await res.json()
    if (res.ok) {
      showToast('Success', data.message, 'success')
      await fetchStatus()
    } else {
      showToast('Error', data, 'error')
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    isInstalling.value = false
  }
}

onMounted(() => {
  fetchStatus()
  pollInterval = setInterval(fetchStatus, 15000)
})

onUnmounted(() => {
  clearInterval(pollInterval)
})
</script>

<template>
  <section class="card">
    <h2 class="card-title"><Cloud class="w-5 h-5 text-brand-500" /> Cloudflare Tunnel</h2>

    <div v-if="isLoading" class="text-sm text-slate-500 flex items-center gap-2">
      <Loader2 class="w-4 h-4 animate-spin" /> Loading status...
    </div>

    <div v-else-if="status" class="space-y-4">
      <!-- Status row -->
      <div class="flex flex-wrap items-center gap-3 p-3 bg-slate-50 rounded-lg border border-slate-200">
        <!-- Binary -->
        <div class="flex items-center gap-1.5">
          <CheckCircle2 v-if="status.installed" class="w-4 h-4 text-green-500" />
          <XCircle v-else class="w-4 h-4 text-red-400" />
          <div>
            <div class="text-xs font-semibold" :class="status.installed ? 'text-green-700' : 'text-red-600'">
              {{ status.installed ? 'Installed' : 'Not Installed' }}
            </div>
            <div v-if="status.version" class="text-[10px] text-slate-400 font-mono">v{{ status.version }}</div>
          </div>
        </div>

        <div class="text-slate-300 hidden sm:block">|</div>

        <!-- Service -->
        <div class="flex items-center gap-1.5">
          <CheckCircle2 v-if="status.service_active" class="w-4 h-4 text-green-500" />
          <XCircle v-else class="w-4 h-4 text-slate-400" />
          <span class="text-xs font-semibold" :class="status.service_active ? 'text-green-700' : 'text-slate-500'">
            {{ status.service_active ? 'Service Active' : 'Service Inactive' }}
          </span>
        </div>

        <div class="text-slate-300 hidden sm:block">|</div>

        <!-- Auth -->
        <div class="flex items-center gap-1.5">
          <ShieldCheck v-if="status.auth_cert_exists" class="w-4 h-4 text-green-500" />
          <AlertTriangle v-else class="w-4 h-4 text-amber-400" />
          <span class="text-xs font-semibold" :class="status.auth_cert_exists ? 'text-green-700' : 'text-amber-600'">
            {{ status.auth_cert_exists ? 'Authorized' : 'Not Authorized' }}
          </span>
        </div>
      </div>

      <!-- Tunnel UUID if config exists -->
      <div v-if="status.config_exists && status.tunnel_uuid" class="text-xs text-slate-500">
        Tunnel UUID: <code class="font-mono bg-slate-100 px-1 rounded">{{ status.tunnel_uuid }}</code>
      </div>

      <!-- Install button -->
      <div v-if="!status.installed">
        <button @click="installCloudflared" class="btn-primary" :disabled="isInstalling">
          <Loader2 v-if="isInstalling" class="w-4 h-4 animate-spin" />
          <DownloadCloud v-else class="w-4 h-4" />
          {{ isInstalling ? 'Installing...' : 'Install Cloudflared' }}
        </button>
      </div>

      <!-- Manage link hint -->
      <p v-if="status.installed" class="text-xs text-slate-400">
        Go to the <strong>Cloudflare</strong> tab to manage tunnels and routes.
      </p>
    </div>
  </section>
</template>
