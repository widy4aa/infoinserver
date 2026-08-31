<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import {
  Cloud, CheckCircle2, XCircle, Loader2, Plus, Trash2,
  RefreshCw, Terminal, KeyRound, ExternalLink, ShieldCheck,
  AlertTriangle, DownloadCloud, ChevronRight, Copy, Play, Square, Globe, Activity
} from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast, showConfirm } = useToastStore()
const { isDark } = useThemeStore()

const handleApiError = async (res) => {
  let errText = ''
  try {
    const data = await res.clone().json()
    errText = data.message || data.error || JSON.stringify(data)
  } catch (e) {
    errText = await res.text()
  }
  showToast('Error', errText, 'error')
}
const status = ref(null)
const isLoadingStatus = ref(true)

// ── Config (ingress routes) ──────────────────────────────────
const localConfig = ref(null)
const isLoadingConfig = ref(false)

// ── Add route form ───────────────────────────────────────────
const newHostname = ref('')
const newService = ref('http://127.0.0.1:')
const isAddingRoute = ref(false)

// ── Login flow ───────────────────────────────────────────────
const loginUrl = ref(null)
const isStartingLogin = ref(false)
const isPollingLogin = ref(false)
let loginPollTimer = null

// ── Create tunnel form ───────────────────────────────────────
const newTunnelName = ref('')
const isCreatingTunnel = ref(false)
const createTunnelResult = ref(null)

// ── Action states ────────────────────────────────────────────
const isRestarting = ref(false)
const isStarting = ref(false)
const isStopping = ref(false)
const isInstalling = ref(false)

// ── Health Diagnostics ───────────────────────────────────────
const healthStatus = ref([])
const isCheckingHealth = ref(false)

const checkHealth = async () => {
  isCheckingHealth.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/health`)
    if (res.ok) {
      healthStatus.value = await res.json()
    } else {
      await handleApiError(res)
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    isCheckingHealth.value = false
  }
}

// ── Computed helpers ─────────────────────────────────────────
const isFullySetup = computed(() =>
  status.value?.installed &&
  status.value?.auth_cert_exists &&
  status.value?.config_exists
)

const routes = computed(() =>
  localConfig.value?.ingress?.filter(r => r.hostname) ?? []
)

// ── Fetch status ─────────────────────────────────────────────
const fetchStatus = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/status`)
    if (res.ok) {
      status.value = await res.json()
      // Jika sudah setup, ambil config
      if (status.value?.config_exists && !localConfig.value) {
        await fetchConfig()
      }
    }
  } catch (e) {
    console.error(e)
  } finally {
    isLoadingStatus.value = false
  }
}

const fetchConfig = async () => {
  isLoadingConfig.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/config`)
    if (res.ok) {
      localConfig.value = await res.json()
    } else {
      const err = await res.text()
      showToast('Error', err, 'error')
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    isLoadingConfig.value = false
  }
}

// ── Install ──────────────────────────────────────────────────
const installCloudflared = async () => {
  isInstalling.value = true
  showToast('Info', 'Downloading and installing cloudflared...')
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/install`, { method: 'POST' })
    if (res.ok) {
      const data = await res.json()
      showToast('Success', data.message, 'success')
      await fetchStatus()
    } else {
      await handleApiError(res)
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    isInstalling.value = false
  }
}

// ── Create Tunnel ────────────────────────────────────────────
const createTunnel = async () => {
  if (!newTunnelName.value.trim()) {
    showToast('Warning', 'Tunnel name cannot be empty', 'warning')
    return
  }
  isCreatingTunnel.value = true
  createTunnelResult.value = null
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/create`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: newTunnelName.value.trim() })
    })
    if (res.ok) {
      const data = await res.json()
      createTunnelResult.value = data
      showToast('Success', data.message, 'success')
      await fetchStatus()
    } else {
      await handleApiError(res)
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    isCreatingTunnel.value = false
  }
}

// ── Delete Tunnel ────────────────────────────────────────────
const isDeletingTunnel = ref(false)
const deleteTunnel = () => {
  showConfirm(
    'Delete Tunnel',
    'Are you sure you want to permanently delete this tunnel? This will break all routed hostnames, remove the tunnel from your Cloudflare account, and clear local configurations.',
    async () => {
      isDeletingTunnel.value = true
      try {
        const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/tunnel`, { method: 'DELETE' })
        if (res.ok) {
          const data = await res.json()
          showToast('Success', data.message, 'success')
          createTunnelResult.value = null // reset result box if any
          localConfig.value = null // reset config
          await fetchStatus()
        } else {
          await handleApiError(res)
        }
      } catch (e) {
        showToast('Error', e.message, 'error')
      } finally {
        isDeletingTunnel.value = false
      }
    }
  )
}

// ── Login / Auth ─────────────────────────────────────────────
const startLogin = async () => {
  isStartingLogin.value = true
  loginUrl.value = null
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/login`, { method: 'POST' })
    if (res.ok) {
      const data = await res.json()
      loginUrl.value = data.url
      showToast('Info', data.message)
      // Mulai polling cert.pem
      startPollingLogin()
    } else {
      await handleApiError(res)
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    isStartingLogin.value = false
  }
}

const startPollingLogin = () => {
  isPollingLogin.value = true
  loginPollTimer = setInterval(async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/login/status`)
      const data = await res.json()
      if (data.authenticated) {
        clearInterval(loginPollTimer)
        isPollingLogin.value = false
        loginUrl.value = null
        showToast('Success', 'Authentication successful! cert.pem saved.', 'success')
        await fetchStatus()
      }
    } catch (e) {
      clearInterval(loginPollTimer)
      isPollingLogin.value = false
    }
  }, 3000)
}

const copyToClipboard = (text) => {
  navigator.clipboard.writeText(text).then(() => {
    showToast('Copied', 'URL copied to clipboard', 'success')
  })
}

// ── Route management ─────────────────────────────────────────
const addRoute = async () => {
  if (!newHostname.value || !newService.value || !localConfig.value?.tunnel) {
    showToast('Warning', 'Hostname, service, and tunnel name are required', 'warning')
    return
  }
  isAddingRoute.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/routes`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        tunnel_name: localConfig.value.tunnel,
        hostname: newHostname.value.trim(),
        service: newService.value.trim()
      })
    })
    if (res.ok) {
      const data = await res.json()
      showToast('Success', data.message, 'success')
      newHostname.value = ''
      newService.value = 'http://127.0.0.1:'
      await fetchConfig()
    } else {
      await handleApiError(res)
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    isAddingRoute.value = false
  }
}

const deleteRoute = (hostname) => {
  showConfirm('Hapus Route', `Yakin ingin menghapus route untuk domain "${hostname}"?`, async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/routes`, {
        method: 'DELETE',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ hostname })
      })
      if (res.ok) {
        const data = await res.json()
        showToast('Success', data.message, 'success')
        await fetchConfig()
      } else {
        await handleApiError(res)
      }
    } catch (e) {
      showToast('Error', e.message, 'error')
    }
  })
}

// ── Register CNAME manually ──────────────────────────────────
const registeringDnsMap = ref({})
const registerDns = async (hostname) => {
  const tunnelName = status.value?.tunnel_name || localConfig.value?.tunnel
  if (!tunnelName) {
    showToast('Error', 'Tunnel name / UUID is required to register DNS CNAME.', 'error')
    return
  }
  registeringDnsMap.value[hostname] = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/routes/dns`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        tunnel_name: tunnelName,
        hostname: hostname
      })
    })
    if (res.ok) {
      const data = await res.json()
      showToast('Success', data.message, 'success')
      // Refresh config agar state cname_active berubah di tabel
      await fetchConfig()
    } else {
      await handleApiError(res)
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    registeringDnsMap.value[hostname] = false
  }
}

// ── Restart / Start / Stop ──────────────────────────────────────────────────
const restartService = async () => {
  isRestarting.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/restart`, { method: 'POST' })
    if (res.ok) {
      const data = await res.json()
      showToast('Success', data.message, 'success')
      await fetchStatus()
    } else {
      await handleApiError(res)
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    isRestarting.value = false
  }
}

const startService = async () => {
  isStarting.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/start`, { method: 'POST' })
    if (res.ok) {
      const data = await res.json()
      showToast('Success', data.message, 'success')
      await fetchStatus()
    } else {
      await handleApiError(res)
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    isStarting.value = false
  }
}

const stopService = async () => {
  showConfirm("Confirm Stop", "Are you sure you want to stop the cloudflared service?", async () => {
    isStopping.value = true
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/stop`, { method: 'POST' })
      if (res.ok) {
        const data = await res.json()
        showToast('Success', data.message, 'success')
        await fetchStatus()
      } else {
        await handleApiError(res)
      }
    } catch (e) {
      showToast('Error', e.message, 'error')
    } finally {
      isStopping.value = false
    }
  })
}

const quickRefCommands = [
  { command: 'sudo systemctl restart cloudflared', description: 'Apply latest ingress rules from config.yml' },
  { command: 'sudo systemctl status cloudflared', description: 'Check if tunnel daemon is active (running)' },
  { command: 'sudo journalctl -u cloudflared -f', description: 'Monitor live traffic & error logs' },
  { command: 'cloudflared tunnel list', description: 'Show UUID and connection status of your tunnel' },
  { command: 'cat /etc/cloudflared/config.yml', description: 'View current hostname routing config' },
  { command: 'cloudflared tunnel login', description: 'Authorize server with Cloudflare (creates cert.pem)' },
]

// ── Logs ─────────────────────────────────────────────────────
const logs = ref([])
const isFetchingLogs = ref(false)
let logsTimer = null

const fetchLogs = async () => {
  if (isFetchingLogs.value) return
  isFetchingLogs.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/logs`)
    if (res.ok) {
      const data = await res.json()
      logs.value = data.logs || []
    }
  } catch (e) {
    console.error("Failed to fetch logs", e)
  } finally {
    isFetchingLogs.value = false
  }
}

// ── Lifecycle ────────────────────────────────────────────────
onMounted(async () => {
  await fetchStatus()
  fetchLogs()
  checkHealth() // Auto check health on load
  logsTimer = setInterval(fetchLogs, 5000)
})

onUnmounted(() => {
  if (logsTimer) clearInterval(logsTimer)
  if (loginPollTimer) clearInterval(loginPollTimer)
})
</script>

<template>
  <div class="space-y-6">

    <!-- ── Loading State ───────────────────────────────────────── -->
    <div v-if="isLoadingStatus" class="flex items-center gap-2 p-6 text-slate-500">
      <Loader2 class="w-5 h-5 animate-spin" />
      <span>Loading Cloudflare status...</span>
    </div>

    <template v-else-if="status">

      <!-- ── Section 1: Status Card ─────────────────────────────── -->
      <section class="card">
        <div class="flex items-start justify-between gap-4 flex-wrap">
          <div>
            <h2 class="card-title">
              <Cloud class="w-5 h-5 text-brand-500" />
              Cloudflare Tunnel — Local Management
            </h2>
            <p class="text-sm text-slate-500 mt-0.5">Manage tunnels via <code class="bg-slate-100 dark:bg-slate-800 px-1 rounded text-xs">/etc/cloudflared/config.yml</code></p>
          </div>
          <div class="flex flex-wrap gap-2">
            <button @click="fetchStatus" class="btn-outline text-xs" title="Refresh status">
              <RefreshCw class="w-4 h-4" />
            </button>
            <button v-if="!status.service_active && !status.running && status.installed" @click="startService" class="btn-success text-xs" :disabled="isStarting">
              <Loader2 v-if="isStarting" class="w-4 h-4 animate-spin" />
              <Play v-else class="w-4 h-4" />
              Start Service
            </button>
            <button v-if="status.service_active || status.running" @click="stopService" class="btn-danger text-xs" :disabled="isStopping">
              <Loader2 v-if="isStopping" class="w-4 h-4 animate-spin" />
              <Square v-else class="w-4 h-4" />
              Stop Service
            </button>
            <button v-if="status.service_active || status.running" @click="restartService" class="btn-outline text-xs" :disabled="isRestarting">
              <Loader2 v-if="isRestarting" class="w-4 h-4 animate-spin" />
              <RefreshCw v-else class="w-4 h-4" />
              Restart
            </button>
          </div>
        </div>

        <!-- Status badges -->
        <div class="mt-4 grid grid-cols-2 sm:grid-cols-4 gap-3">
          <!-- Installed -->
          <div class="flex flex-col gap-1 p-3 rounded-lg border" :class="status.installed ? (isDark ? 'border-green-800 bg-green-900/20' : 'border-green-200 bg-green-50') : (isDark ? 'border-red-800 bg-red-900/20' : 'border-red-200 bg-red-50')">
            <span class="text-[10px] font-bold uppercase tracking-wider" :class="status.installed ? (isDark ? 'text-green-400' : 'text-green-600') : (isDark ? 'text-red-400' : 'text-red-600')">Binary</span>
            <div class="flex items-center gap-1.5">
              <CheckCircle2 v-if="status.installed" class="w-4 h-4 text-green-500" />
              <XCircle v-else class="w-4 h-4 text-red-400" />
              <span class="text-xs font-semibold" :class="status.installed ? (isDark ? 'text-green-300' : 'text-green-700') : (isDark ? 'text-red-300' : 'text-red-600')">
                {{ status.installed ? 'Installed' : 'Not Installed' }}
              </span>
            </div>
            <span v-if="status.version" class="text-[10px] font-mono text-slate-500">v{{ status.version }}</span>
          </div>

          <!-- Service Active -->
          <div class="flex flex-col gap-1 p-3 rounded-lg border" :class="status.service_active ? (isDark ? 'border-green-800 bg-green-900/20' : 'border-green-200 bg-green-50') : (isDark ? 'border-slate-700 bg-slate-800/50' : 'border-slate-200 bg-slate-50')">
            <span class="text-[10px] font-bold uppercase tracking-wider" :class="status.service_active ? (isDark ? 'text-green-400' : 'text-green-600') : 'text-slate-500'">Service</span>
            <div class="flex items-center gap-1.5">
              <CheckCircle2 v-if="status.service_active" class="w-4 h-4 text-green-500" />
              <XCircle v-else class="w-4 h-4 text-slate-400" />
              <span class="text-xs font-semibold" :class="status.service_active ? (isDark ? 'text-green-300' : 'text-green-700') : (isDark ? 'text-slate-400' : 'text-slate-600')">
                {{ status.service_active ? 'Active' : 'Inactive' }}
              </span>
            </div>
          </div>

          <!-- Auth -->
          <div class="flex flex-col gap-1 p-3 rounded-lg border" :class="status.auth_cert_exists ? (isDark ? 'border-green-800 bg-green-900/20' : 'border-green-200 bg-green-50') : (isDark ? 'border-amber-800 bg-amber-900/20' : 'border-amber-200 bg-amber-50')">
            <span class="text-[10px] font-bold uppercase tracking-wider" :class="status.auth_cert_exists ? (isDark ? 'text-green-400' : 'text-green-600') : (isDark ? 'text-amber-400' : 'text-amber-600')">Auth</span>
            <div class="flex items-center gap-1.5">
              <ShieldCheck v-if="status.auth_cert_exists" class="w-4 h-4 text-green-500" />
              <AlertTriangle v-else class="w-4 h-4 text-amber-400" />
              <span class="text-xs font-semibold" :class="status.auth_cert_exists ? (isDark ? 'text-green-300' : 'text-green-700') : (isDark ? 'text-amber-300' : 'text-amber-700')">
                {{ status.auth_cert_exists ? 'Authorized' : 'Not Authorized' }}
              </span>
            </div>
            <span class="text-[10px] text-slate-500">cert.pem</span>
          </div>

          <!-- Config / Tunnel -->
          <div class="flex flex-col gap-1 p-3 rounded-lg border relative" :class="status.config_exists ? (isDark ? 'border-green-800 bg-green-900/20' : 'border-green-200 bg-green-50') : (isDark ? 'border-slate-700 bg-slate-800/50' : 'border-slate-200 bg-slate-50')">
            <div class="flex justify-between items-start">
              <span class="text-[10px] font-bold uppercase tracking-wider" :class="status.config_exists ? (isDark ? 'text-green-400' : 'text-green-600') : 'text-slate-500'">Tunnel Config</span>
              <button v-if="status.config_exists" @click="deleteTunnel" class="text-slate-400 hover:text-red-500 p-0.5" title="Delete Tunnel" :disabled="isDeletingTunnel">
                <Loader2 v-if="isDeletingTunnel" class="w-3.5 h-3.5 animate-spin" />
                <Trash2 v-else class="w-3.5 h-3.5" />
              </button>
            </div>
            <div class="flex items-center gap-1.5">
              <CheckCircle2 v-if="status.config_exists" class="w-4 h-4 text-green-500" />
              <XCircle v-else class="w-4 h-4 text-slate-400" />
              <span class="text-xs font-semibold" :class="status.config_exists ? (isDark ? 'text-green-300' : 'text-green-700') : (isDark ? 'text-slate-400' : 'text-slate-600')">
                {{ status.tunnel_name || (status.config_exists ? 'Unnamed Tunnel' : 'Not Found') }}
              </span>
            </div>
            <span v-if="status.tunnel_uuid" class="text-[10px] font-mono text-slate-500 truncate" :title="status.tunnel_uuid">UUID: {{ status.tunnel_uuid.substring(0,8) }}...</span>
          </div>
        </div>

        <!-- Install button jika belum install -->
        <div v-if="!status.installed" class="mt-4">
          <button @click="installCloudflared" class="btn-primary" :disabled="isInstalling">
            <Loader2 v-if="isInstalling" class="w-4 h-4 animate-spin" />
            <DownloadCloud v-else class="w-4 h-4" />
            {{ isInstalling ? 'Installing...' : 'Install cloudflared' }}
          </button>
        </div>
      </section>

      <!-- ── Section 2: Setup Steps (jika belum fully setup) ──────── -->
      <section v-if="status.installed && !isFullySetup" class="card space-y-6">
        <h2 class="card-title">
          <Terminal class="w-5 h-5 text-brand-500" />
          Setup — First Time Configuration
        </h2>

        <!-- Step 1: Authorize -->
        <div class="space-y-3">
          <div class="flex items-center gap-2">
            <span class="w-6 h-6 rounded-full text-xs font-bold flex items-center justify-center"
              :class="status.auth_cert_exists ? (isDark ? 'bg-green-900/30 text-green-400' : 'bg-green-100 text-green-700') : (isDark ? 'bg-brand-900/30 text-brand-400' : 'bg-brand-100 text-brand-700')">
              {{ status.auth_cert_exists ? '✓' : '1' }}
            </span>
            <h3 class="font-semibold text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-800'">Authorize with Cloudflare</h3>
          </div>
          <p class="text-xs ml-8" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
            Runs <code class="px-1 rounded" :class="isDark ? 'bg-slate-800' : 'bg-slate-100'">cloudflared tunnel login</code> — opens a Cloudflare URL to authorize your server. After authorization, <code class="px-1 rounded" :class="isDark ? 'bg-slate-800' : 'bg-slate-100'">cert.pem</code> is saved automatically.
          </p>

          <div v-if="status.auth_cert_exists" class="ml-8 flex items-center gap-2 text-sm" :class="isDark ? 'text-green-400' : 'text-green-600'">
            <CheckCircle2 class="w-4 h-4" /> Already authorized
          </div>

          <div v-else class="ml-8 space-y-3">
            <button @click="startLogin" class="btn-primary" :disabled="isStartingLogin || isPollingLogin">
              <Loader2 v-if="isStartingLogin || isPollingLogin" class="w-4 h-4 animate-spin" />
              <KeyRound v-else class="w-4 h-4" />
              {{ isStartingLogin ? 'Starting...' : isPollingLogin ? 'Waiting for authorization...' : 'Start Cloudflare Login' }}
            </button>

            <!-- Authorization URL box -->
            <div v-if="loginUrl" class="p-3 border rounded-lg" :class="isDark ? 'bg-amber-900/10 border-amber-800' : 'bg-amber-50 border-amber-200'">
              <p class="text-xs font-semibold mb-2" :class="isDark ? 'text-amber-400' : 'text-amber-700'">Open this URL in your browser and authorize your domain:</p>
              <div class="flex items-start gap-2">
                <a :href="loginUrl" target="_blank" class="text-xs font-mono hover:underline break-all flex-1" :class="isDark ? 'text-brand-400' : 'text-brand-700'">
                  {{ loginUrl }}
                </a>
                <div class="flex gap-1 flex-shrink-0">
                  <button @click="copyToClipboard(loginUrl)" class="p-1.5 rounded" :class="isDark ? 'hover:bg-amber-900/30 text-amber-500' : 'hover:bg-amber-100 text-amber-600'" title="Copy URL">
                    <Copy class="w-3.5 h-3.5" />
                  </button>
                  <a :href="loginUrl" target="_blank" class="p-1.5 rounded" :class="isDark ? 'hover:bg-amber-900/30 text-amber-500' : 'hover:bg-amber-100 text-amber-600'" title="Open in new tab">
                    <ExternalLink class="w-3.5 h-3.5" />
                  </a>
                </div>
              </div>
              <p v-if="isPollingLogin" class="text-[10px] mt-2 flex items-center gap-1" :class="isDark ? 'text-amber-500' : 'text-amber-600'">
                <Loader2 class="w-3 h-3 animate-spin" />
                Waiting for you to authorize in the browser...
              </p>
            </div>
          </div>
        </div>

        <hr :class="isDark ? 'border-slate-700' : 'border-slate-200'" />

        <!-- Step 2: Create Tunnel -->
        <div class="space-y-3">
          <div class="flex items-center gap-2">
            <span class="w-6 h-6 rounded-full text-xs font-bold flex items-center justify-center"
              :class="status.config_exists ? (isDark ? 'bg-green-900/30 text-green-400' : 'bg-green-100 text-green-700') : (isDark ? 'bg-brand-900/30 text-brand-400' : 'bg-brand-100 text-brand-700')">
              {{ status.config_exists ? '✓' : '2' }}
            </span>
            <h3 class="font-semibold text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-800'">Create a Named Tunnel</h3>
          </div>
          <p class="text-xs ml-8" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
            Runs <code class="px-1 rounded" :class="isDark ? 'bg-slate-800' : 'bg-slate-100'">cloudflared tunnel create &lt;name&gt;</code> — generates a tunnel UUID and credentials file. The dashboard will automatically create <code class="px-1 rounded" :class="isDark ? 'bg-slate-800' : 'bg-slate-100'">/etc/cloudflared/config.yml</code> for you.
          </p>

          <div v-if="status.config_exists" class="ml-8 flex items-center gap-2 text-sm" :class="isDark ? 'text-green-400' : 'text-green-600'">
            <CheckCircle2 class="w-4 h-4" /> Tunnel configured: {{ status.tunnel_name || 'unknown' }} <span class="text-xs opacity-75">(UUID: {{ status.tunnel_uuid ?? 'unknown' }})</span>
          </div>

          <div v-else class="ml-8 space-y-3">
            <div class="flex gap-2">
              <input
                v-model="newTunnelName"
                type="text"
                placeholder="e.g. my-server-1"
                class="input-field max-w-xs"
                :disabled="isCreatingTunnel"
              />
              <button @click="createTunnel" class="btn-primary" :disabled="isCreatingTunnel">
                <Loader2 v-if="isCreatingTunnel" class="w-4 h-4 animate-spin" />
                <Plus v-else class="w-4 h-4" />
                {{ isCreatingTunnel ? 'Creating...' : 'Create Tunnel' }}
              </button>
            </div>

            <!-- Output dari create tunnel -->
            <div v-if="createTunnelResult" class="p-3 border rounded-lg" :class="isDark ? 'bg-green-900/10 border-green-800' : 'bg-green-50 border-green-200'">
              <p class="text-xs font-semibold mb-1" :class="isDark ? 'text-green-400' : 'text-green-700'">{{ createTunnelResult.message }}</p>
              <p v-if="createTunnelResult.uuid" class="text-xs font-mono" :class="isDark ? 'text-slate-300' : 'text-slate-700'">
                UUID: <strong>{{ createTunnelResult.uuid }}</strong>
              </p>
              <p class="text-xs mt-2" :class="isDark ? 'text-green-500' : 'text-green-600'">
                <strong>Config automatically generated!</strong> You can now proceed to add routes below.
              </p>
            </div>
          </div>
        </div>
      </section>

      <!-- ── Section 3 & 4: Split Layout (Ingress & DNS) ──── -->
      <div v-if="status.config_exists" class="grid grid-cols-1 lg:grid-cols-2 gap-6 items-start">
        
        <!-- ── Section 3: Ingress Routes ──── -->
        <section class="card">
          <div class="flex items-center justify-between gap-4 flex-wrap mb-4">
            <h2 class="card-title">
              <ChevronRight class="w-5 h-5 text-brand-500" />
              Ingress Routes
              <span v-if="localConfig" class="text-xs font-normal ml-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
                (Tunnel: <code class="font-mono">{{ status?.tunnel_name || localConfig.tunnel }}</code>)
              </span>
            </h2>
            <button @click="fetchConfig" class="btn-outline text-xs" :disabled="isLoadingConfig">
              <Loader2 v-if="isLoadingConfig" class="w-3.5 h-3.5 animate-spin" />
              <RefreshCw v-else class="w-3.5 h-3.5" />
              Sync
            </button>
          </div>

          <!-- Add route form -->
          <div class="border rounded-lg p-4 mb-5" :class="isDark ? 'bg-slate-800/50 border-slate-700' : 'bg-slate-50 border-slate-200'">
            <h3 class="text-sm font-semibold mb-3" :class="isDark ? 'text-slate-200' : 'text-slate-700'">Add New Route</h3>
            <div class="flex flex-col xl:flex-row gap-2">
              <div class="flex flex-col gap-2 flex-1">
                <input
                  v-model="newHostname"
                  type="text"
                  placeholder="Public hostname (e.g. app.widy4aa.my.id)"
                  class="input-field w-full"
                  :disabled="isAddingRoute"
                />
                <input
                  v-model="newService"
                  type="text"
                  placeholder="Local service (e.g. http://127.0.0.1:8080)"
                  class="input-field w-full"
                  :disabled="isAddingRoute"
                />
              </div>
              <button @click="addRoute" class="btn-primary whitespace-nowrap self-end xl:self-auto xl:h-auto py-2" :disabled="isAddingRoute">
                <Loader2 v-if="isAddingRoute" class="w-4 h-4 animate-spin" />
                <Plus v-else class="w-4 h-4" />
                {{ isAddingRoute ? 'Adding...' : 'Add Route' }}
              </button>
            </div>
            <p class="text-[10px] mt-2" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
              Adding a route here will only update local <code>config.yml</code>. You must add the CNAME manually in the DNS table.
            </p>
          </div>

          <!-- Routes table -->
          <div v-if="isLoadingConfig" class="flex justify-center py-8">
            <Loader2 class="w-6 h-6 animate-spin text-brand-500" />
          </div>

          <div v-else-if="localConfig" class="overflow-x-auto">
            <table class="w-full">
              <thead class="border-b-2" :class="isDark ? 'bg-slate-800/50 border-slate-700' : 'bg-slate-50 border-slate-200'">
                <tr>
                  <th class="table-th">Public Hostname</th>
                  <th class="table-th">Local Service</th>
                  <th class="table-th text-right w-16">Action</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="r in routes" :key="r.hostname" class="border-b transition-colors" :class="isDark ? 'hover:bg-slate-800/50 border-slate-700' : 'hover:bg-slate-50 border-slate-100'">
                  <td class="table-td font-semibold" :class="isDark ? 'text-brand-400' : 'text-brand-700'">
                    <a :href="`https://${r.hostname}`" target="_blank" class="hover:underline flex items-center gap-1">
                      {{ r.hostname }}
                      <ExternalLink class="w-3 h-3" :class="isDark ? 'text-slate-500' : 'text-slate-400'" />
                    </a>
                  </td>
                  <td class="table-td font-mono text-xs" :class="isDark ? 'text-slate-300' : 'text-slate-600'">{{ r.service }}</td>
                  <td class="table-td text-right">
                    <button
                      @click="deleteRoute(r.hostname)"
                      class="p-1.5 rounded"
                      :class="isDark ? 'text-red-400 hover:text-red-300 hover:bg-red-900/30' : 'text-red-400 hover:text-red-600 hover:bg-red-50'"
                      title="Remove Route from Config"
                    >
                      <Trash2 class="w-4 h-4" />
                    </button>
                  </td>
                </tr>
                <tr v-if="routes.length === 0">
                  <td colspan="3" class="text-center p-6 text-sm" :class="isDark ? 'text-slate-500' : 'text-slate-500'">
                    No ingress routes configured. Add one above.
                  </td>
                </tr>
              </tbody>
            </table>
            <!-- Catch-all info -->
            <div class="mt-2 px-2 text-[10px]" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
              Fallback: <code>http_status:404</code> (always last)
            </div>
          </div>
        </section>

      <!-- ── Section 4: CNAME DNS Records ────────────────────────── -->
      <section class="card">
        <h2 class="card-title">
          <Globe class="w-5 h-5 text-brand-500" />
          CNAME DNS Records
        </h2>
        <p class="text-[10px] mb-4" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
          Manage Cloudflare DNS routing for the hostnames defined in your ingress configuration.
        </p>
        
        <div v-if="isLoadingConfig" class="flex justify-center py-8">
          <Loader2 class="w-6 h-6 animate-spin text-brand-500" />
        </div>

        <div v-else-if="localConfig" class="overflow-x-auto">
          <table class="w-full">
            <thead class="border-b-2" :class="isDark ? 'bg-slate-800/50 border-slate-700' : 'bg-slate-50 border-slate-200'">
              <tr>
                <th class="table-th">Hostname</th>
                <th class="table-th w-32 text-left">Status</th>
                <th class="table-th text-right w-24">Action</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="r in routes" :key="r.hostname" class="border-b transition-colors" :class="isDark ? 'hover:bg-slate-800/50 border-slate-700' : 'hover:bg-slate-50 border-slate-100'">
                <td class="table-td font-semibold" :class="isDark ? 'text-brand-400' : 'text-brand-700'">
                  {{ r.hostname }}
                </td>
                <td class="table-td text-left whitespace-nowrap">
                  <span v-if="r.cname_active" class="px-2 py-0.5 rounded text-[10px] font-semibold bg-green-100 text-green-800 dark:bg-green-950/40 dark:text-green-400 border border-green-200 dark:border-green-800">
                    <CheckCircle2 class="w-3 h-3 inline mr-1" /> Active
                  </span>
                  <span v-else class="px-2 py-0.5 rounded text-[10px] font-semibold bg-amber-100 text-amber-800 dark:bg-amber-950/40 dark:text-amber-400 border border-amber-200 dark:border-amber-800">
                    <AlertTriangle class="w-3 h-3 inline mr-1" /> Not Registered
                  </span>
                </td>
                <td class="table-td text-right">
                  <button
                    v-if="!r.cname_active"
                    @click="registerDns(r.hostname)"
                    class="btn-primary py-1 px-2 text-[10px] ml-auto whitespace-nowrap"
                    title="Add CNAME to Cloudflare"
                    :disabled="registeringDnsMap[r.hostname]"
                  >
                    <Loader2 v-if="registeringDnsMap[r.hostname]" class="w-3 h-3 animate-spin" />
                    <Plus v-else class="w-3 h-3" />
                    Add CNAME
                  </button>
                  <span v-else class="text-[10px] text-slate-400 italic">Managed</span>
                </td>
              </tr>
              <tr v-if="routes.length === 0">
                <td colspan="3" class="text-center p-6 text-sm" :class="isDark ? 'text-slate-500' : 'text-slate-500'">
                  No hostnames found to register.
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
      </div>

      <!-- ── Section 4.5: Route Health Diagnostics ────────────────── -->
      <section v-if="status.config_exists && healthStatus.length > 0" class="card">
        <div class="flex items-center justify-between mb-4">
          <div>
            <h2 class="card-title">
              <Activity class="w-5 h-5 text-brand-500" />
              Route Health Diagnostics
            </h2>
            <p class="text-[10px] mt-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
              HTTP probes to verify if your domain is reachable from the outside world.
            </p>
          </div>
          <button @click="checkHealth" class="btn-outline text-xs whitespace-nowrap" :disabled="isCheckingHealth">
            <Loader2 v-if="isCheckingHealth" class="w-3.5 h-3.5 animate-spin" />
            <RefreshCw v-else class="w-3.5 h-3.5" />
            Run Diagnostics
          </button>
        </div>

        <div class="overflow-x-auto">
          <table class="w-full">
            <thead class="border-b-2" :class="isDark ? 'bg-slate-800/50 border-slate-700' : 'bg-slate-50 border-slate-200'">
              <tr>
                <th class="table-th w-1/3">Hostname</th>
                <th class="table-th">End-to-End Status</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="h in healthStatus" :key="h.hostname" class="border-b transition-colors" :class="isDark ? 'hover:bg-slate-800/50 border-slate-700' : 'hover:bg-slate-50 border-slate-100'">
                <td class="table-td font-semibold" :class="isDark ? 'text-brand-400' : 'text-brand-700'">
                  {{ h.hostname }}
                </td>
                <td class="table-td">
                  <div class="flex items-center gap-2">
                    <span v-if="h.code === 'HEALTHY'" class="px-2 py-0.5 rounded text-[10px] font-semibold bg-green-100 text-green-800 dark:bg-green-950/40 dark:text-green-400 border border-green-200 dark:border-green-800">
                      <CheckCircle2 class="w-3 h-3 inline mr-1" /> {{ h.status }}
                    </span>
                    <span v-else-if="h.code === 'ERR_502'" class="px-2 py-0.5 rounded text-[10px] font-semibold bg-red-100 text-red-800 dark:bg-red-950/40 dark:text-red-400 border border-red-200 dark:border-red-800">
                      <XCircle class="w-3 h-3 inline mr-1" /> {{ h.status }}
                    </span>
                    <span v-else-if="h.code === 'ERR_1033'" class="px-2 py-0.5 rounded text-[10px] font-semibold bg-amber-100 text-amber-800 dark:bg-amber-950/40 dark:text-amber-400 border border-amber-200 dark:border-amber-800">
                      <AlertTriangle class="w-3 h-3 inline mr-1" /> {{ h.status }}
                    </span>
                    <span v-else class="px-2 py-0.5 rounded text-[10px] font-semibold bg-slate-100 text-slate-800 dark:bg-slate-800 dark:text-slate-300 border border-slate-200 dark:border-slate-700">
                      <Activity class="w-3 h-3 inline mr-1" /> {{ h.status }}
                    </span>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <!-- ── Section 5: Quick Reference ────────────────────────── -->
      <section v-if="status.installed" class="card">
        <h2 class="card-title">
          <Terminal class="w-5 h-5 text-brand-500" />
          Quick Reference — Useful Commands
        </h2>
        <div class="mt-3 overflow-x-auto">
          <table class="w-full text-xs">
            <thead class="border-b" :class="isDark ? 'bg-slate-800/50 border-slate-700' : 'bg-slate-50 border-slate-200'">
              <tr>
                <th class="text-left px-3 py-2 font-semibold" :class="isDark ? 'text-slate-300' : 'text-slate-600'">Command</th>
                <th class="text-left px-3 py-2 font-semibold" :class="isDark ? 'text-slate-300' : 'text-slate-600'">Description</th>
              </tr>
            </thead>
            <tbody class="divide-y" :class="isDark ? 'divide-slate-700' : 'divide-slate-100'">
              <tr v-for="cmd in quickRefCommands" :key="cmd.command" :class="isDark ? 'hover:bg-slate-800/50' : 'hover:bg-slate-50'">
                <td class="px-3 py-2.5 font-mono whitespace-nowrap" :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ cmd.command }}</td>
                <td class="px-3 py-2.5" :class="isDark ? 'text-slate-400' : 'text-slate-500'">{{ cmd.description }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <!-- ── Section 5: Cloudflare Logs ────────────────────────── -->
      <section v-if="status.installed" class="card">
        <div class="flex items-center justify-between mb-3">
          <h2 class="card-title">
            <Terminal class="w-5 h-5 text-brand-500" />
            Live Logs
          </h2>
          <div class="flex items-center gap-2">
            <span v-if="isFetchingLogs" class="text-xs flex items-center gap-1 text-slate-500">
              <Loader2 class="w-3 h-3 animate-spin" /> Fetching...
            </span>
            <button @click="fetchLogs" class="btn-outline text-xs" :disabled="isFetchingLogs">
              <RefreshCw class="w-3.5 h-3.5" />
              Refresh Logs
            </button>
          </div>
        </div>
        <div class="bg-black/90 text-green-400 font-mono text-[11px] p-4 rounded-lg overflow-x-auto h-64 overflow-y-auto whitespace-pre font-medium shadow-inner leading-relaxed">
          <div v-if="logs.length === 0" class="text-slate-500 italic">No logs found...</div>
          <div v-for="(line, idx) in logs" :key="idx" :class="{'text-red-400': line.includes('ERR'), 'text-amber-300': line.includes('WRN')}">
            {{ line }}
          </div>
        </div>
      </section>

    </template>
  </div>
</template>
