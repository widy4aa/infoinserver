<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import {
  Cloud, CheckCircle2, XCircle, Loader2, Plus, Trash2,
  RefreshCw, Terminal, KeyRound, ExternalLink, ShieldCheck,
  AlertTriangle, DownloadCloud, ChevronRight, Copy, Play, Square,
  Globe, Activity, Zap, Clock, Link2, Wifi, WifiOff
} from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl, getToken, activeServerId } = useServerStore()
const { showToast, showConfirm } = useToastStore()
const { isDark } = useThemeStore()

// ── STATE ────────────────────────────────────────────────────
const status = ref(null)
const isLoadingStatus = ref(true)
const localConfig = ref(null)
const isLoadingConfig = ref(false)

const activeTab = ref('health') // 'health' | 'routes' | 'logs'

// Route form
const newHostname = ref('')
const newService = ref('http://127.0.0.1:')
const isAddingRoute = ref(false)

// Tunnel setup
const loginUrl = ref(null)
const isStartingLogin = ref(false)
const isPollingLogin = ref(false)
let loginPollTimer = null
const newTunnelName = ref('')
const isCreatingTunnel = ref(false)

// Service controls
const isRestarting = ref(false)
const isStarting = ref(false)
const isStopping = ref(false)
const isInstalling = ref(false)
const isDeletingTunnel = ref(false)

// Health
const healthStatus = ref([])
const isCheckingHealth = ref(false)

// CNAME DNS
const registeringDnsMap = ref({})

// Live Logs
const logs = ref([])
const isFetchingLogs = ref(false)
const logsContainer = ref(null)
const logFilter = ref('all') // 'all' | 'err' | 'wrn'
const isLogsPaused = ref(false)
let ws = null

// ── COMPUTED ──────────────────────────────────────────────────
const isFullySetup = computed(() =>
  status.value?.installed &&
  status.value?.auth_cert_exists &&
  status.value?.config_exists
)

const routes = computed(() =>
  localConfig.value?.ingress?.filter(r => r.hostname) ?? []
)

const dnsActiveCount = computed(() =>
  routes.value.filter(r => r.cname_active).length
)

const filteredLogs = computed(() => {
  if (logFilter.value === 'err') return logs.value.filter(l => l.includes('ERR') || l.includes('error'))
  if (logFilter.value === 'wrn') return logs.value.filter(l => l.includes('WRN') || l.includes('warn'))
  return logs.value
})

const formatUptime = (secs) => {
  if (!secs) return '—'
  const d = Math.floor(secs / 86400)
  const h = Math.floor((secs % 86400) / 3600)
  const m = Math.floor((secs % 3600) / 60)
  if (d > 0) return `${d}d ${h}h`
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

const tunnelUuidShort = computed(() => {
  const uuid = status.value?.tunnel_uuid
  return uuid ? uuid.substring(0, 8) + '...' : '—'
})

// ── API HELPERS ───────────────────────────────────────────────
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

const fetchStatus = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/status`)
    if (res.ok) {
      status.value = await res.json()
      if (status.value?.config_exists && !localConfig.value) {
        await fetchConfig()
      }
    }
  } catch (e) { console.error(e) }
  finally { isLoadingStatus.value = false }
}

const fetchConfig = async () => {
  isLoadingConfig.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/config`)
    if (res.ok) localConfig.value = await res.json()
    else showToast('Error', await res.text(), 'error')
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally { isLoadingConfig.value = false }
}

// ── INSTALL ───────────────────────────────────────────────────
const installCloudflared = async () => {
  isInstalling.value = true
  showToast('Info', 'Downloading and installing cloudflared...')
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/install`, { method: 'POST' })
    if (res.ok) { showToast('Success', (await res.json()).message, 'success'); await fetchStatus() }
    else await handleApiError(res)
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { isInstalling.value = false }
}

// ── TUNNEL SETUP ──────────────────────────────────────────────
const createTunnel = async () => {
  if (!newTunnelName.value.trim()) return showToast('Warning', 'Tunnel name cannot be empty', 'warning')
  isCreatingTunnel.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/create`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: newTunnelName.value.trim() })
    })
    if (res.ok) { showToast('Success', (await res.json()).message, 'success'); await fetchStatus() }
    else await handleApiError(res)
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { isCreatingTunnel.value = false }
}

const deleteTunnel = () => {
  showConfirm('Delete Tunnel', 'This will permanently remove the tunnel from Cloudflare and clear all local configurations.', async () => {
    isDeletingTunnel.value = true
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/tunnel`, { method: 'DELETE' })
      if (res.ok) { showToast('Success', (await res.json()).message, 'success'); localConfig.value = null; await fetchStatus() }
      else await handleApiError(res)
    } catch (e) { showToast('Error', e.message, 'error') }
    finally { isDeletingTunnel.value = false }
  })
}

// ── LOGIN / AUTH ──────────────────────────────────────────────
const startLogin = async () => {
  isStartingLogin.value = true; loginUrl.value = null
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/login`, { method: 'POST' })
    if (res.ok) {
      const data = await res.json(); loginUrl.value = data.url
      showToast('Info', data.message)
      isPollingLogin.value = true
      loginPollTimer = setInterval(async () => {
        try {
          const r = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/login/status`)
          const d = await r.json()
          if (d.authenticated) {
            clearInterval(loginPollTimer); isPollingLogin.value = false; loginUrl.value = null
            showToast('Success', 'Authentication successful! cert.pem saved.', 'success')
            await fetchStatus()
          }
        } catch (e) { clearInterval(loginPollTimer); isPollingLogin.value = false }
      }, 3000)
    } else await handleApiError(res)
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { isStartingLogin.value = false }
}

const copyToClipboard = (text) => {
  navigator.clipboard.writeText(text).then(() => showToast('Copied', 'Copied to clipboard', 'success'))
}

// ── SERVICE CONTROLS ──────────────────────────────────────────
const startService = async () => {
  isStarting.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/start`, { method: 'POST' })
    if (res.ok) { showToast('Success', (await res.json()).message, 'success'); await fetchStatus() }
    else await handleApiError(res)
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { isStarting.value = false }
}

const stopService = () => {
  showConfirm('Confirm Stop', 'Are you sure you want to stop the cloudflared service?', async () => {
    isStopping.value = true
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/stop`, { method: 'POST' })
      if (res.ok) { showToast('Success', (await res.json()).message, 'success'); await fetchStatus() }
      else await handleApiError(res)
    } catch (e) { showToast('Error', e.message, 'error') }
    finally { isStopping.value = false }
  })
}

const restartService = async () => {
  isRestarting.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/restart`, { method: 'POST' })
    if (res.ok) { showToast('Success', (await res.json()).message, 'success'); await fetchStatus() }
    else await handleApiError(res)
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { isRestarting.value = false }
}

// ── ROUTES ────────────────────────────────────────────────────
const addRoute = async () => {
  if (!newHostname.value || !newService.value || !localConfig.value?.tunnel)
    return showToast('Warning', 'Hostname, service, and tunnel name are required', 'warning')
  isAddingRoute.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/routes`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ tunnel_name: localConfig.value.tunnel, hostname: newHostname.value.trim(), service: newService.value.trim() })
    })
    if (res.ok) {
      showToast('Success', (await res.json()).message, 'success')
      newHostname.value = ''; newService.value = 'http://127.0.0.1:'
      await fetchConfig(); checkHealth()
    } else await handleApiError(res)
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { isAddingRoute.value = false }
}

const deleteRoute = (hostname) => {
  showConfirm('Delete Route', `Delete route for "${hostname}"?`, async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/routes`, {
        method: 'DELETE', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ hostname })
      })
      if (res.ok) { showToast('Success', (await res.json()).message, 'success'); await fetchConfig(); checkHealth() }
      else await handleApiError(res)
    } catch (e) { showToast('Error', e.message, 'error') }
  })
}

const registerDns = async (hostname) => {
  const tunnelName = status.value?.tunnel_name || localConfig.value?.tunnel
  if (!tunnelName) return showToast('Error', 'Tunnel name required to register CNAME.', 'error')
  registeringDnsMap.value[hostname] = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/routes/dns`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ tunnel_name: tunnelName, hostname })
    })
    if (res.ok) { showToast('Success', (await res.json()).message, 'success'); await fetchConfig(); checkHealth() }
    else await handleApiError(res)
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { registeringDnsMap.value[hostname] = false }
}

// ── HEALTH ────────────────────────────────────────────────────
const checkHealth = async () => {
  isCheckingHealth.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/health`)
    if (res.ok) healthStatus.value = await res.json()
    else await handleApiError(res)
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { isCheckingHealth.value = false }
}

const healthCodeColor = (code) => {
  if (code === 'HEALTHY') return { bg: 'bg-emerald-500/10 border-emerald-500/20', text: 'text-emerald-600 dark:text-emerald-400', icon: 'text-emerald-500', dot: 'bg-emerald-400' }
  if (code === 'ERR_502') return { bg: 'bg-red-500/10 border-red-500/20', text: 'text-red-600 dark:text-red-400', icon: 'text-red-500', dot: 'bg-red-400' }
  if (code === 'ERR_1033') return { bg: 'bg-amber-500/10 border-amber-500/20', text: 'text-amber-600 dark:text-amber-400', icon: 'text-amber-500', dot: 'bg-amber-400' }
  if (code === 'NXDOMAIN') return { bg: 'bg-slate-500/10 border-slate-500/20', text: 'text-slate-500', icon: 'text-slate-400', dot: 'bg-slate-400' }
  return { bg: 'bg-slate-500/10 border-slate-500/20', text: 'text-slate-500', icon: 'text-slate-400', dot: 'bg-slate-400' }
}

const healthCodeLabel = (code) => {
  if (code === 'HEALTHY') return 'Accessible'
  if (code === 'ERR_502') return 'Bad Gateway (502)'
  if (code === 'ERR_1033') return 'Tunnel Down (1033)'
  if (code === 'NXDOMAIN') return 'DNS Not Found'
  if (code === 'TIMEOUT') return 'Timeout'
  return 'Unknown Error'
}

const healthCodeDescription = (code) => {
  if (code === 'HEALTHY') return 'Domain is reachable and responding normally.'
  if (code === 'ERR_502') return 'Tunnel is connected to Cloudflare but the local service is down. Check your application.'
  if (code === 'ERR_1033') return 'Cloudflare cannot reach the cloudflared daemon. Make sure the service is running.'
  if (code === 'NXDOMAIN') return 'DNS CNAME record not found. Register the CNAME in the Routes tab first.'
  if (code === 'TIMEOUT') return 'Request timed out. The server may be overloaded or unreachable.'
  return code
}

// ── LIVE LOGS ─────────────────────────────────────────────────
const connectLogsWs = () => {
  if (ws) ws.close()
  isFetchingLogs.value = true
  logs.value = []
  let wsUrl = getActiveServerUrl().replace(/^http/, 'ws') + '/api/cloudflare/logs/ws'
  const token = getToken(activeServerId.value)
  if (token) wsUrl += '?token=' + encodeURIComponent(token)
  ws = new WebSocket(wsUrl)
  ws.onopen = () => { isFetchingLogs.value = false; logs.value.push('--- Connected ---') }
  ws.onmessage = (event) => {
    if (isLogsPaused.value) return
    logs.value.push(event.data)
    if (logs.value.length > 500) logs.value.shift()
    setTimeout(() => { if (logsContainer.value) logsContainer.value.scrollTop = logsContainer.value.scrollHeight }, 10)
  }
  ws.onclose = () => { isFetchingLogs.value = false; logs.value.push('--- Disconnected ---') }
  ws.onerror = () => { isFetchingLogs.value = false }
}

const clearLogs = () => { logs.value = [] }

// ── LIFECYCLE ─────────────────────────────────────────────────
onMounted(async () => {
  await fetchStatus()
  if (status.value?.installed) connectLogsWs()
  checkHealth()
})

onUnmounted(() => {
  if (ws) ws.close()
  if (loginPollTimer) clearInterval(loginPollTimer)
})
</script>

<template>
  <div>
    <!-- Loading -->
    <div v-if="isLoadingStatus" class="flex items-center justify-center py-24 text-slate-500">
      <Loader2 class="w-6 h-6 animate-spin mr-2" /> Loading Cloudflare status...
    </div>

    <template v-else-if="status">

      <!-- ══ SETUP WIZARD ══ (jika belum fully setup) -->
      <div v-if="!isFullySetup" class="min-h-[60vh] flex flex-col items-center justify-center">
        <div class="w-full max-w-2xl">

          <!-- Header -->
          <div class="text-center mb-10">
            <div class="w-16 h-16 bg-orange-100 dark:bg-orange-900/30 rounded-2xl flex items-center justify-center mx-auto mb-4">
              <Cloud class="w-8 h-8 text-orange-500" />
            </div>
            <h1 class="text-2xl font-bold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">Setup Cloudflare Tunnel</h1>
            <p class="text-slate-500 text-sm mt-1">Connect your server to Cloudflare's global network</p>
          </div>

          <!-- Stepper -->
          <div class="flex items-center justify-center gap-0 mb-8">
            <div v-for="(step, i) in [
              { label: 'Install', done: status.installed },
              { label: 'Authorize', done: status.auth_cert_exists },
              { label: 'Create Tunnel', done: status.config_exists }
            ]" :key="i" class="flex items-center">
              <div class="flex flex-col items-center">
                <div class="w-9 h-9 rounded-full flex items-center justify-center text-sm font-bold transition-all"
                     :class="step.done
                       ? 'bg-emerald-500 text-white'
                       : i === [status.installed, status.auth_cert_exists, status.config_exists].filter(Boolean).length
                         ? 'bg-orange-500 text-white ring-4 ring-orange-200 dark:ring-orange-900/40'
                         : (isDark ? 'bg-slate-700 text-slate-400' : 'bg-slate-200 text-slate-500')">
                  <CheckCircle2 v-if="step.done" class="w-4 h-4" />
                  <span v-else>{{ i + 1 }}</span>
                </div>
                <span class="text-[10px] mt-1 font-semibold" :class="step.done ? 'text-emerald-500' : (isDark ? 'text-slate-400' : 'text-slate-500')">
                  {{ step.label }}
                </span>
              </div>
              <div v-if="i < 2" class="w-24 h-0.5 mb-5" :class="step.done ? 'bg-emerald-400' : (isDark ? 'bg-slate-700' : 'bg-slate-200')"></div>
            </div>
          </div>

          <!-- Active Step Content -->
          <div class="card border-2" :class="isDark ? 'border-slate-700' : 'border-slate-200'">

            <!-- Step 1: Install -->
            <div v-if="!status.installed">
              <div class="flex items-start gap-4 mb-6">
                <div class="w-10 h-10 bg-blue-100 dark:bg-blue-900/30 rounded-xl flex items-center justify-center shrink-0">
                  <DownloadCloud class="w-5 h-5 text-blue-500" />
                </div>
                <div>
                  <h2 class="font-bold text-lg">Install cloudflared Binary</h2>
                  <p class="text-sm text-slate-500 mt-1">Downloads and installs the official cloudflared binary from GitHub to <code class="text-xs bg-slate-100 dark:bg-slate-800 px-1 rounded">/usr/local/bin</code></p>
                </div>
              </div>
              <button @click="installCloudflared" class="btn-primary w-full justify-center py-3 text-base" :disabled="isInstalling">
                <Loader2 v-if="isInstalling" class="w-5 h-5 animate-spin" />
                <DownloadCloud v-else class="w-5 h-5" />
                {{ isInstalling ? 'Installing...' : 'Install cloudflared' }}
              </button>
            </div>

            <!-- Step 2: Authorize -->
            <div v-else-if="!status.auth_cert_exists">
              <div class="flex items-start gap-4 mb-6">
                <div class="w-10 h-10 bg-amber-100 dark:bg-amber-900/30 rounded-xl flex items-center justify-center shrink-0">
                  <KeyRound class="w-5 h-5 text-amber-500" />
                </div>
                <div>
                  <h2 class="font-bold text-lg">Authorize with Cloudflare</h2>
                  <p class="text-sm text-slate-500 mt-1">Runs <code class="text-xs bg-slate-100 dark:bg-slate-800 px-1 rounded">cloudflared tunnel login</code> to link this server to your Cloudflare account. A URL will be generated — open it in your browser.</p>
                </div>
              </div>
              <button @click="startLogin" class="btn-primary w-full justify-center py-3 text-base mb-4" :disabled="isStartingLogin || isPollingLogin">
                <Loader2 v-if="isStartingLogin || isPollingLogin" class="w-5 h-5 animate-spin" />
                <KeyRound v-else class="w-5 h-5" />
                {{ isStartingLogin ? 'Starting...' : isPollingLogin ? 'Waiting for authorization...' : 'Start Cloudflare Login' }}
              </button>

              <!-- Login URL box -->
              <div v-if="loginUrl" class="mt-4 p-4 rounded-xl border-2 border-amber-200 dark:border-amber-800 bg-amber-50 dark:bg-amber-900/10">
                <p class="text-xs font-bold text-amber-700 dark:text-amber-400 mb-2 flex items-center gap-1.5">
                  <ExternalLink class="w-3.5 h-3.5" /> Open this URL in your browser to authorize:
                </p>
                <div class="flex items-center gap-2">
                  <a :href="loginUrl" target="_blank" class="text-xs font-mono break-all text-brand-600 dark:text-brand-400 hover:underline flex-1">{{ loginUrl }}</a>
                  <button @click="copyToClipboard(loginUrl)" class="p-2 rounded-lg text-amber-600 hover:bg-amber-100 dark:hover:bg-amber-900/30 shrink-0">
                    <Copy class="w-4 h-4" />
                  </button>
                  <a :href="loginUrl" target="_blank" class="p-2 rounded-lg text-amber-600 hover:bg-amber-100 dark:hover:bg-amber-900/30 shrink-0">
                    <ExternalLink class="w-4 h-4" />
                  </a>
                </div>
                <p v-if="isPollingLogin" class="text-[10px] mt-3 text-amber-600 dark:text-amber-500 flex items-center gap-1">
                  <Loader2 class="w-3 h-3 animate-spin" /> Waiting for authorization in browser...
                </p>
              </div>
            </div>

            <!-- Step 3: Create Tunnel -->
            <div v-else-if="!status.config_exists">
              <div class="flex items-start gap-4 mb-6">
                <div class="w-10 h-10 bg-purple-100 dark:bg-purple-900/30 rounded-xl flex items-center justify-center shrink-0">
                  <Cloud class="w-5 h-5 text-purple-500" />
                </div>
                <div>
                  <h2 class="font-bold text-lg">Create a Named Tunnel</h2>
                  <p class="text-sm text-slate-500 mt-1">Creates a persistent tunnel with a unique ID. The config file will be auto-generated at <code class="text-xs bg-slate-100 dark:bg-slate-800 px-1 rounded">/etc/cloudflared/config.yml</code></p>
                </div>
              </div>
              <div class="flex gap-3">
                <input v-model="newTunnelName" type="text" placeholder="Tunnel name (e.g. my-server)" class="input-field flex-1" :disabled="isCreatingTunnel" @keyup.enter="createTunnel" />
                <button @click="createTunnel" class="btn-primary px-6 shrink-0" :disabled="isCreatingTunnel">
                  <Loader2 v-if="isCreatingTunnel" class="w-4 h-4 animate-spin" />
                  <Plus v-else class="w-4 h-4" />
                  {{ isCreatingTunnel ? 'Creating...' : 'Create' }}
                </button>
              </div>
            </div>

          </div>
        </div>
      </div>

      <!-- ══ COMMAND CENTER ══ (jika fully setup) -->
      <div v-else class="flex gap-6 h-[calc(100vh-160px)]">

        <!-- ─ LEFT: Control Panel ─ -->
        <div class="w-72 shrink-0 flex flex-col gap-4 overflow-y-auto pb-2">

          <!-- Tunnel Identity -->
          <div class="card !p-5 bg-gradient-to-br from-orange-500/5 to-transparent border-orange-200/50 dark:border-orange-800/30">
            <!-- Status + Name -->
            <div class="flex items-start justify-between mb-3">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 mb-1">
                  <div class="relative">
                    <div class="w-2.5 h-2.5 rounded-full" :class="status.service_active ? 'bg-emerald-400' : 'bg-slate-400'"></div>
                    <div v-if="status.service_active" class="absolute inset-0 rounded-full bg-emerald-400 animate-ping opacity-60"></div>
                  </div>
                  <span class="text-[10px] font-bold uppercase tracking-wider" :class="status.service_active ? 'text-emerald-500' : (isDark ? 'text-slate-400' : 'text-slate-500')">
                    {{ status.service_active ? 'Running' : 'Stopped' }}
                  </span>
                </div>
                <h2 class="text-xl font-bold leading-tight truncate" :class="isDark ? 'text-slate-100' : 'text-slate-800'" :title="status.tunnel_name">
                  {{ status.tunnel_name || 'Unnamed Tunnel' }}
                </h2>
                <div class="text-[10px] text-slate-500 font-mono mt-0.5">v{{ status.version || '—' }}</div>
              </div>
              <button @click="fetchStatus" class="p-1.5 rounded-lg text-slate-400 hover:text-slate-600 hover:bg-slate-100 dark:hover:bg-slate-700 shrink-0 ml-2" title="Refresh">
                <RefreshCw class="w-3.5 h-3.5" />
              </button>
            </div>

            <!-- UUID -->
            <div class="flex items-center gap-2 bg-slate-100 dark:bg-slate-800/60 rounded-lg px-3 py-1.5 mt-2">
              <span class="text-[10px] font-mono text-slate-500 flex-1 truncate">{{ status.tunnel_uuid || 'No UUID' }}</span>
              <button v-if="status.tunnel_uuid" @click="copyToClipboard(status.tunnel_uuid)" class="text-slate-400 hover:text-slate-600 shrink-0">
                <Copy class="w-3 h-3" />
              </button>
            </div>
          </div>

          <!-- Quick Stats -->
          <div class="grid grid-cols-3 gap-2">
            <div class="card !p-3 text-center">
              <div class="text-xl font-bold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">{{ routes.length }}</div>
              <div class="text-[9px] font-bold uppercase tracking-wider text-slate-500 mt-0.5">Routes</div>
            </div>
            <div class="card !p-3 text-center">
              <div class="text-xl font-bold text-emerald-500">{{ dnsActiveCount }}</div>
              <div class="text-[9px] font-bold uppercase tracking-wider text-slate-500 mt-0.5">DNS Live</div>
            </div>
            <div class="card !p-3 text-center">
              <div class="text-sm font-bold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">{{ formatUptime(status.service_uptime_secs) }}</div>
              <div class="text-[9px] font-bold uppercase tracking-wider text-slate-500 mt-0.5">Uptime</div>
            </div>
          </div>

          <!-- Status Pills -->
          <div class="card !p-4 space-y-2.5">
            <h3 class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-3">System Status</h3>
            <div v-for="(pill, label) in {
              'Binary': { ok: status.installed, detail: status.version ? 'v' + status.version : 'Not installed' },
              'Service': { ok: status.service_active, detail: status.service_active ? 'Active' : 'Inactive' },
              'Auth': { ok: status.auth_cert_exists, detail: status.auth_cert_exists ? 'cert.pem found' : 'Not authorized' },
              'Config': { ok: status.config_exists, detail: status.config_exists ? 'jail.local found' : 'No config' },
            }" :key="label" class="flex items-center gap-3">
              <div class="w-5 h-5 rounded-full flex items-center justify-center shrink-0"
                   :class="pill.ok ? 'bg-emerald-100 dark:bg-emerald-900/30' : 'bg-red-100 dark:bg-red-900/30'">
                <CheckCircle2 v-if="pill.ok" class="w-3 h-3 text-emerald-500" />
                <XCircle v-else class="w-3 h-3 text-red-400" />
              </div>
              <div class="flex-1 min-w-0">
                <div class="text-xs font-semibold" :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ label }}</div>
                <div class="text-[10px] text-slate-500 truncate">{{ pill.detail }}</div>
              </div>
            </div>
          </div>

          <!-- Service Controls -->
          <div class="card !p-4">
            <h3 class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-3">Service Control</h3>
            <div class="flex flex-col gap-2">
              <button v-if="!status.service_active && !status.running" @click="startService" class="btn-success justify-center w-full" :disabled="isStarting">
                <Loader2 v-if="isStarting" class="w-4 h-4 animate-spin" /><Play v-else class="w-4 h-4" />
                {{ isStarting ? 'Starting...' : 'Start Service' }}
              </button>
              <button v-if="status.service_active || status.running" @click="restartService" class="btn-outline justify-center w-full" :disabled="isRestarting">
                <Loader2 v-if="isRestarting" class="w-4 h-4 animate-spin" /><RefreshCw v-else class="w-4 h-4" />
                {{ isRestarting ? 'Restarting...' : 'Restart' }}
              </button>
              <button v-if="status.service_active || status.running" @click="stopService" class="btn-danger justify-center w-full" :disabled="isStopping">
                <Loader2 v-if="isStopping" class="w-4 h-4 animate-spin" /><Square v-else class="w-4 h-4" />
                {{ isStopping ? 'Stopping...' : 'Stop Service' }}
              </button>
            </div>
          </div>

          <!-- Danger Zone -->
          <div class="card !p-4 border-red-200 dark:border-red-800/40 bg-red-50/50 dark:bg-red-900/5 mt-auto">
            <div class="text-[10px] font-bold uppercase tracking-wider text-red-500 mb-2">Danger Zone</div>
            <button @click="deleteTunnel" class="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg border border-red-300 dark:border-red-700 text-red-600 dark:text-red-400 hover:bg-red-100 dark:hover:bg-red-900/30 text-xs font-semibold transition-colors" :disabled="isDeletingTunnel">
              <Loader2 v-if="isDeletingTunnel" class="w-3.5 h-3.5 animate-spin" />
              <Trash2 v-else class="w-3.5 h-3.5" />
              {{ isDeletingTunnel ? 'Deleting...' : 'Delete Tunnel' }}
            </button>
          </div>
        </div>

        <!-- ─ RIGHT: Tab Panel ─ -->
        <div class="flex-1 flex flex-col min-w-0 overflow-hidden">

          <!-- Tab Header -->
          <div class="flex items-center gap-1 border-b mb-4 shrink-0" :class="isDark ? 'border-slate-800' : 'border-slate-200'">
            <button v-for="tab in [
              { id: 'health', icon: 'Activity', label: 'Health' },
              { id: 'routes', icon: 'ChevronRight', label: 'Routes' },
              { id: 'logs', icon: 'Terminal', label: 'Live Logs' }
            ]" :key="tab.id" @click="activeTab = tab.id"
              class="px-4 py-2.5 text-sm font-semibold border-b-2 transition-colors"
              :class="activeTab === tab.id
                ? 'border-brand-500 text-brand-600 dark:text-brand-400'
                : 'border-transparent text-slate-500 hover:text-slate-700 dark:hover:text-slate-300'">
              <div class="flex items-center gap-2">
                <Activity v-if="tab.id === 'health'" class="w-4 h-4" />
                <ChevronRight v-else-if="tab.id === 'routes'" class="w-4 h-4" />
                <Terminal v-else class="w-4 h-4" />
                {{ tab.label }}
                <span v-if="tab.id === 'health' && healthStatus.length > 0" class="px-1.5 py-0.5 rounded-full text-[10px] font-bold"
                      :class="healthStatus.some(h => h.code !== 'HEALTHY') ? 'bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400' : 'bg-emerald-100 text-emerald-600 dark:bg-emerald-900/30 dark:text-emerald-400'">
                  {{ healthStatus.filter(h => h.code !== 'HEALTHY').length || '✓' }}
                </span>
                <span v-if="tab.id === 'routes'" class="px-1.5 py-0.5 rounded-full text-[10px] font-bold bg-slate-100 dark:bg-slate-800 text-slate-500">
                  {{ routes.length }}
                </span>
              </div>
            </button>
          </div>

          <!-- ── TAB: HEALTH ── -->
          <div v-if="activeTab === 'health'" class="flex-1 overflow-y-auto space-y-3">
            <div class="flex items-center justify-between mb-4">
              <div>
                <h3 class="font-bold text-sm">End-to-End Route Diagnostics</h3>
                <p class="text-[11px] text-slate-500 mt-0.5">HTTP probes to verify each domain is reachable from the internet</p>
              </div>
              <button @click="checkHealth" class="btn-outline text-xs" :disabled="isCheckingHealth">
                <Loader2 v-if="isCheckingHealth" class="w-3.5 h-3.5 animate-spin" />
                <RefreshCw v-else class="w-3.5 h-3.5" />
                {{ isCheckingHealth ? 'Checking...' : 'Run Diagnostics' }}
              </button>
            </div>

            <div v-if="healthStatus.length === 0 && !isCheckingHealth" class="text-center py-16 text-slate-400">
              <Activity class="w-10 h-10 mx-auto mb-3 opacity-30" />
              <p class="text-sm">No diagnostics run yet.</p>
              <p class="text-xs mt-1">Click "Run Diagnostics" to check your routes.</p>
            </div>

            <div v-for="h in healthStatus" :key="h.hostname"
              class="rounded-xl border p-4 transition-all"
              :class="healthCodeColor(h.code).bg">
              <div class="flex items-start justify-between gap-3">
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-1.5">
                    <div class="w-2 h-2 rounded-full shrink-0" :class="healthCodeColor(h.code).dot"></div>
                    <span class="text-sm font-bold truncate" :class="isDark ? 'text-slate-100' : 'text-slate-800'">{{ h.hostname }}</span>
                    <span class="px-2 py-0.5 rounded-full text-[10px] font-bold"
                          :class="`${healthCodeColor(h.code).bg} ${healthCodeColor(h.code).text} border ${healthCodeColor(h.code).bg.replace('bg-', 'border-')}`">
                      {{ healthCodeLabel(h.code) }}
                    </span>
                  </div>
                  <p class="text-xs" :class="isDark ? 'text-slate-400' : 'text-slate-600'">{{ healthCodeDescription(h.code) }}</p>
                </div>
                <div class="flex items-center gap-2 shrink-0">
                  <a v-if="h.code === 'HEALTHY'" :href="`https://${h.hostname}`" target="_blank"
                    class="flex items-center gap-1 px-2 py-1 rounded-lg text-[10px] font-semibold text-emerald-600 hover:bg-emerald-100 dark:hover:bg-emerald-900/30 transition-colors">
                    <ExternalLink class="w-3 h-3" /> Open
                  </a>
                </div>
              </div>
            </div>
          </div>

          <!-- ── TAB: ROUTES ── -->
          <div v-if="activeTab === 'routes'" class="flex-1 overflow-y-auto flex flex-col">
            <!-- Add Route Form -->
            <div class="rounded-xl border p-4 mb-4 shrink-0" :class="isDark ? 'border-slate-700 bg-slate-800/30' : 'border-slate-200 bg-slate-50'">
              <h3 class="text-xs font-bold text-slate-500 uppercase tracking-wider mb-3">Add New Route</h3>
              <div class="flex flex-col sm:flex-row gap-2">
                <input v-model="newHostname" type="text" placeholder="Public hostname (e.g. app.example.com)" class="input-field sm:flex-1 text-sm" :disabled="isAddingRoute" />
                <input v-model="newService" type="text" placeholder="Local service (e.g. http://127.0.0.1:8080)" class="input-field sm:flex-1 text-sm" :disabled="isAddingRoute" />
                <button @click="addRoute" class="btn-primary shrink-0" :disabled="isAddingRoute">
                  <Loader2 v-if="isAddingRoute" class="w-4 h-4 animate-spin" /><Plus v-else class="w-4 h-4" />
                  {{ isAddingRoute ? 'Adding...' : 'Add' }}
                </button>
              </div>
            </div>

            <!-- Routes Table -->
            <div class="flex-1 overflow-auto rounded-xl border" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
              <table class="w-full">
                <thead class="sticky top-0 text-xs" :class="isDark ? 'bg-slate-800 border-b border-slate-700 text-slate-400' : 'bg-slate-50 border-b border-slate-200 text-slate-500'">
                  <tr>
                    <th class="px-4 py-3 text-left font-semibold">Hostname</th>
                    <th class="px-4 py-3 text-left font-semibold">Local Service</th>
                    <th class="px-4 py-3 text-center font-semibold w-28">DNS Status</th>
                    <th class="px-4 py-3 text-right font-semibold w-36">Actions</th>
                  </tr>
                </thead>
                <tbody class="divide-y" :class="isDark ? 'divide-slate-800' : 'divide-slate-100'">
                  <tr v-for="r in routes" :key="r.hostname"
                    class="transition-colors" :class="isDark ? 'hover:bg-slate-800/50' : 'hover:bg-slate-50/80'">
                    <td class="px-4 py-3">
                      <div class="flex items-center gap-2">
                        <Globe class="w-3.5 h-3.5 text-slate-400 shrink-0" />
                        <a :href="`https://${r.hostname}`" target="_blank"
                          class="text-sm font-semibold hover:underline flex items-center gap-1"
                          :class="isDark ? 'text-brand-400' : 'text-brand-700'">
                          {{ r.hostname }}
                          <ExternalLink class="w-2.5 h-2.5 opacity-60" />
                        </a>
                      </div>
                    </td>
                    <td class="px-4 py-3 font-mono text-xs" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
                      {{ r.service }}
                    </td>
                    <td class="px-4 py-3 text-center">
                      <span v-if="r.cname_active"
                        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400">
                        <CheckCircle2 class="w-3 h-3" /> Active
                      </span>
                      <span v-else class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400">
                        <AlertTriangle class="w-3 h-3" /> Pending
                      </span>
                    </td>
                    <td class="px-4 py-3">
                      <div class="flex items-center justify-end gap-1.5">
                        <button v-if="!r.cname_active" @click="registerDns(r.hostname)"
                          class="px-2.5 py-1 rounded-lg text-[10px] font-bold bg-brand-50 hover:bg-brand-100 text-brand-600 border border-brand-200 dark:bg-brand-900/20 dark:text-brand-400 dark:border-brand-800 transition-colors"
                          :disabled="registeringDnsMap[r.hostname]">
                          <Loader2 v-if="registeringDnsMap[r.hostname]" class="w-3 h-3 animate-spin inline" />
                          {{ registeringDnsMap[r.hostname] ? '...' : 'Add CNAME' }}
                        </button>
                        <a :href="`https://${r.hostname}`" target="_blank"
                          class="p-1.5 rounded-lg text-slate-400 hover:text-brand-500 hover:bg-brand-50 dark:hover:bg-brand-900/20 transition-colors" title="Open URL">
                          <ExternalLink class="w-3.5 h-3.5" />
                        </a>
                        <button @click="deleteRoute(r.hostname)"
                          class="p-1.5 rounded-lg text-slate-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors" title="Delete Route">
                          <Trash2 class="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </td>
                  </tr>
                  <tr v-if="routes.length === 0">
                    <td colspan="4" class="text-center py-12 text-slate-400 text-sm">
                      <Globe class="w-8 h-8 mx-auto mb-2 opacity-30" />
                      No routes configured. Add one above.
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p class="text-[10px] text-slate-500 mt-2 px-1">Fallback: <code>http_status:404</code> (always last in config.yml)</p>
          </div>

          <!-- ── TAB: LOGS ── -->
          <div v-if="activeTab === 'logs'" class="flex-1 flex flex-col min-h-0">
            <!-- Log Toolbar -->
            <div class="flex items-center gap-3 mb-3 shrink-0">
              <div class="flex items-center gap-1.5 text-xs">
                <div class="w-1.5 h-1.5 rounded-full" :class="ws && ws.readyState === 1 ? 'bg-emerald-400 animate-pulse' : 'bg-slate-400'"></div>
                <span class="text-slate-500">{{ ws && ws.readyState === 1 ? 'Connected' : 'Disconnected' }}</span>
              </div>
              <div class="flex items-center gap-1 ml-auto">
                <select v-model="logFilter" class="text-xs border rounded-lg px-2 py-1" :class="isDark ? 'bg-slate-800 border-slate-700 text-slate-300' : 'bg-white border-slate-200 text-slate-700'">
                  <option value="all">All Logs</option>
                  <option value="err">Errors Only</option>
                  <option value="wrn">Warnings Only</option>
                </select>
                <button @click="isLogsPaused = !isLogsPaused"
                  class="px-2.5 py-1 rounded-lg text-xs border font-semibold transition-colors"
                  :class="isLogsPaused ? 'bg-amber-100 border-amber-200 text-amber-700 dark:bg-amber-900/20 dark:border-amber-800 dark:text-amber-400' : (isDark ? 'border-slate-700 text-slate-400 hover:bg-slate-800' : 'border-slate-200 text-slate-600 hover:bg-slate-100')">
                  {{ isLogsPaused ? 'Resume' : 'Pause' }}
                </button>
                <button @click="clearLogs" class="px-2.5 py-1 rounded-lg text-xs border font-semibold" :class="isDark ? 'border-slate-700 text-slate-400 hover:bg-slate-800' : 'border-slate-200 text-slate-600 hover:bg-slate-100'">
                  Clear
                </button>
                <button @click="connectLogsWs" class="px-2.5 py-1 rounded-lg text-xs border font-semibold" :class="isDark ? 'border-slate-700 text-slate-400 hover:bg-slate-800' : 'border-slate-200 text-slate-600 hover:bg-slate-100'" :disabled="isFetchingLogs">
                  <Loader2 v-if="isFetchingLogs" class="w-3 h-3 animate-spin inline" />
                  {{ isFetchingLogs ? '' : 'Reconnect' }}
                </button>
              </div>
            </div>

            <!-- Terminal -->
            <div ref="logsContainer" class="flex-1 rounded-xl bg-black/90 p-4 font-mono text-[11px] overflow-y-auto scroll-smooth" style="min-height: 0">
              <div v-if="filteredLogs.length === 0" class="text-slate-600 italic">No logs to display...</div>
              <div v-for="(line, idx) in filteredLogs" :key="idx" class="leading-relaxed py-0.5"
                :class="{
                  'text-red-400': line.includes('ERR') || line.includes('error'),
                  'text-amber-300': line.includes('WRN') || line.includes('warn'),
                  'text-slate-500 italic': line.startsWith('---'),
                  'text-emerald-400': !line.includes('ERR') && !line.includes('WRN') && !line.startsWith('---')
                }">{{ line }}</div>
            </div>
          </div>

        </div>
      </div>

    </template>
  </div>
</template>
