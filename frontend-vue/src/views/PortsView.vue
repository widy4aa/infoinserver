<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { Network, Radar, ShieldCheck, Activity, Play, Download, Upload, Server,
  CheckCircle2, XCircle, AlertTriangle, Plus, Trash2, Loader2, RefreshCw,
  Wifi, WifiOff, Globe, Search, Filter, ToggleLeft, ToggleRight, Ban } from 'lucide-vue-next'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast, showConfirm } = useToastStore()
const { isDark } = useThemeStore()

// ── TABS ─────────────────────────────────────────────────────
const activeTab = ref('network')

// ── NETWORK INTERFACES ───────────────────────────────────────
const networkInterfaces = ref([])
const isLoadingNetwork = ref(true)

const fetchNetwork = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/network`)
    networkInterfaces.value = await res.json()
  } catch (e) {}
  finally { isLoadingNetwork.value = false }
}

const formatBytes = (bytes) => {
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB'
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + ' MB'
  return (bytes / 1073741824).toFixed(2) + ' GB'
}

// ── SPEEDTEST ─────────────────────────────────────────────────
const speedHistory = ref([])
const isRunningSpeedtest = ref(false)
const isLoadingSpeedtest = ref(true)

const latestSpeed = computed(() => speedHistory.value[0] || null)

const fetchSpeedHistory = async () => {
  try {
    isLoadingSpeedtest.value = true
    const res = await apiFetch(`${getActiveServerUrl()}/api/speedtest/history`)
    if (res.ok) speedHistory.value = await res.json()
  } catch (e) {} finally { isLoadingSpeedtest.value = false }
}

const runSpeedtest = async () => {
  isRunningSpeedtest.value = true
  showToast('Info', 'Starting Speedtest... This may take a minute.')
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/speedtest/run`, { method: 'POST' })
    if (res.ok) {
      showToast('Success', 'Speedtest completed', 'success')
      await fetchSpeedHistory()
    } else {
      showToast('Error', `Speedtest failed: ${await res.text()}`, 'error')
    }
  } catch (e) { showToast('Error', 'Failed to run speedtest', 'error') }
  finally { isRunningSpeedtest.value = false }
}

const formatMbps = (mbps) => (!mbps ? '—' : parseFloat(mbps).toFixed(1) + ' Mbps')
const formatDate = (iso) => (!iso ? '—' : new Date(iso).toLocaleString())

// ── UFW FIREWALL ──────────────────────────────────────────────
const ufwStatus = ref(null)
const isLoadingUfw = ref(true)
const isTogglingUfw = ref(false)
const newRule = ref({ action: 'allow', port: '' })
const isAddingRule = ref(false)

const fetchUfw = async () => {
  try {
    isLoadingUfw.value = true
    const res = await apiFetch(`${getActiveServerUrl()}/api/firewall/status`)
    if (res.ok) ufwStatus.value = await res.json()
  } catch (e) {} finally { isLoadingUfw.value = false }
}

const toggleUfw = async () => {
  isTogglingUfw.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/firewall/toggle`, { method: 'POST' })
    if (res.ok) {
      showToast('Success', (await res.json()).message, 'success')
      await fetchUfw()
    } else {
      showToast('Error', await res.text(), 'error')
    }
  } catch (e) { showToast('Error', 'Failed to toggle UFW', 'error') }
  finally { isTogglingUfw.value = false }
}

const addUfwRule = async () => {
  if (!newRule.value.port.trim()) return
  isAddingRule.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/firewall/rule`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(newRule.value)
    })
    if (res.ok) {
      showToast('Success', (await res.json()).message, 'success')
      newRule.value.port = ''
      await fetchUfw()
    } else {
      showToast('Error', await res.text(), 'error')
    }
  } catch (e) { showToast('Error', 'Failed to add rule', 'error') }
  finally { isAddingRule.value = false }
}

const deleteUfwRule = async (port) => {
  showConfirm('Delete Rule', `Delete rule for port ${port}?`, async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/firewall/rule`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: 'delete', port })
      })
      if (res.ok) {
        showToast('Success', 'Rule deleted', 'success')
        await fetchUfw()
      } else {
        showToast('Error', await res.text(), 'error')
      }
    } catch (e) { showToast('Error', 'Failed to delete rule', 'error') }
  })
}

// ── FAIL2BAN ──────────────────────────────────────────────────
const f2bStatus = ref(null)
const f2bLogs = ref([])
const f2bConfig = ref([])
const f2bFilters = ref([])
const isLoadingF2b = ref(true)
const isInstallingF2b = ref(false)
const f2bActiveTab = ref('logs')

const formManualBan = ref({ jail: '', ip: '' })
const isBanning = ref(false)

const showConfigModal = ref(false)
const formConfig = ref({ name: '', enabled: false, port: '', logpath: '', filter: '', maxretry: '', bantime: '', findtime: '' })
const isDeletingJail = ref(false)

const configPreview = computed(() => {
  const f = formConfig.value
  if (!f.name) return ''
  let lines = [`[${f.name}]`, `enabled = ${f.enabled}`]
  if (f.port) lines.push(`port = ${f.port}`)
  if (f.filter) lines.push(`filter = ${f.filter}`)
  if (f.logpath) lines.push(`logpath = ${f.logpath}`)
  if (f.maxretry) lines.push(`maxretry = ${f.maxretry}`)
  if (f.findtime) lines.push(`findtime = ${f.findtime}`)
  if (f.bantime) lines.push(`bantime = ${f.bantime}`)
  return lines.join('\n')
})

const totalBanned = computed(() => f2bStatus.value?.jails?.reduce((sum, j) => sum + j.banned_ips.length, 0) || 0)

const SERVICE_TEMPLATES = [
  { icon: '🔒', name: 'sshd', label: 'SSH', port: 'ssh', filter: 'sshd', logpath: '/var/log/auth.log', maxretry: '5', bantime: '10m', findtime: '10m' },
  { icon: '🌐', name: 'nginx-http-auth', label: 'Nginx HTTP Auth', port: 'http,https', filter: 'nginx-http-auth', logpath: '/var/log/nginx/error.log', maxretry: '5', bantime: '1h', findtime: '10m' },
  { icon: '🕵️', name: 'nginx-botsearch', label: 'Nginx Bot Search', port: 'http,https', filter: 'nginx-botsearch', logpath: '/var/log/nginx/access.log', maxretry: '2', bantime: '1d', findtime: '30m' },
  { icon: '📧', name: 'postfix', label: 'Postfix Mail', port: 'smtp,submission,smtps', filter: 'postfix', logpath: '/var/log/mail.log', maxretry: '5', bantime: '1h', findtime: '10m' },
  { icon: '📦', name: 'dovecot', label: 'Dovecot IMAP', port: 'imap,imaps,pop3,pop3s', filter: 'dovecot', logpath: '/var/log/mail.log', maxretry: '5', bantime: '1h', findtime: '10m' },
  { icon: '🔧', name: 'apache-auth', label: 'Apache Auth', port: 'http,https', filter: 'apache-auth', logpath: '/var/log/apache2/error.log', maxretry: '5', bantime: '1h', findtime: '10m' },
  { icon: '🖥️', name: 'wordpress', label: 'WordPress Login', port: 'http,https', filter: 'wordpress', logpath: '/var/log/nginx/access.log', maxretry: '5', bantime: '24h', findtime: '10m' },
]

const fetchFail2Ban = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/status`)
    if (res.ok) {
      f2bStatus.value = await res.json()
      if (f2bStatus.value?.installed && formManualBan.value.jail === '' && f2bStatus.value.jails?.length > 0) {
        formManualBan.value.jail = f2bStatus.value.jails[0].name
      }
    }
  } catch (e) {} finally { isLoadingF2b.value = false }
}

const fetchF2bLogs = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/logs`)
    if (res.ok) f2bLogs.value = await res.json()
  } catch (e) {}
}

const fetchF2bConfig = async () => {
  try {
    const [configRes, filtersRes] = await Promise.all([
      apiFetch(`${getActiveServerUrl()}/api/fail2ban/config`),
      apiFetch(`${getActiveServerUrl()}/api/fail2ban/filters`)
    ])
    if (configRes.ok) f2bConfig.value = await configRes.json()
    if (filtersRes.ok) f2bFilters.value = await filtersRes.json()
  } catch (e) {}
}

const openConfigModal = async () => {
  await fetchF2bConfig()
  showConfigModal.value = true
}

const installFail2Ban = async () => {
  isInstallingF2b.value = true
  showToast('Info', 'Installing Fail2Ban... This may take a minute.')
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/install`, { method: 'POST' })
    if (res.ok) { showToast('Success', 'Fail2Ban installed and started successfully', 'success'); await fetchFail2Ban() }
    else showToast('Error', await res.text(), 'error')
  } catch (e) { showToast('Error', 'Installation failed', 'error') }
  finally { isInstallingF2b.value = false }
}

const banIp = async () => {
  if (!formManualBan.value.ip || !formManualBan.value.jail) return
  isBanning.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/ban`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(formManualBan.value)
    })
    if (res.ok) { showToast('Success', 'IP has been banned', 'success'); formManualBan.value.ip = ''; await fetchFail2Ban() }
    else showToast('Error', await res.text(), 'error')
  } catch (e) { showToast('Error', 'Failed to ban IP', 'error') }
  finally { isBanning.value = false }
}

const unbanIp = async (jail, ip) => {
  showConfirm('Unban IP', `Unban IP ${ip} from jail ${jail}?`, async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/unban`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ jail, ip })
      })
      if (res.ok) { showToast('Success', 'IP unbanned', 'success'); await fetchFail2Ban() }
      else showToast('Error', await res.text(), 'error')
    } catch (e) { showToast('Error', 'Failed to unban', 'error') }
  })
}

const editJail = (jail) => { formConfig.value = { ...jail } }
const createNewJail = () => {
  formConfig.value = { name: 'custom-', enabled: true, port: '', logpath: '', filter: '', maxretry: '5', bantime: '10m', findtime: '10m' }
}
const applyTemplate = (t) => {
  formConfig.value = { name: t.name, enabled: true, port: t.port, filter: t.filter, logpath: t.logpath, maxretry: t.maxretry, bantime: t.bantime, findtime: t.findtime }
}
const saveJailConfig = async () => {
  if (!formConfig.value.name) return
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/config`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(formConfig.value)
    })
    if (res.ok) {
      showToast('Success', 'Jail saved and reloaded', 'success')
      await fetchF2bConfig(); await fetchFail2Ban()
      formConfig.value = { name: '', enabled: false, port: '', logpath: '', filter: '', maxretry: '', bantime: '', findtime: '' }
    } else showToast('Error', await res.text(), 'error')
  } catch (e) { showToast('Error', 'Failed to save', 'error') }
}
const deleteJail = async () => {
  showConfirm('Delete Jail', `Delete jail "${formConfig.value.name}"?`, async () => {
    isDeletingJail.value = true
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/config/${formConfig.value.name}`, { method: 'DELETE' })
      if (res.ok) {
        showToast('Success', `Jail deleted`, 'success')
        formConfig.value = { name: '', enabled: false, port: '', logpath: '', filter: '', maxretry: '', bantime: '', findtime: '' }
        await fetchF2bConfig(); await fetchFail2Ban()
      } else showToast('Error', await res.text(), 'error')
    } catch (e) { showToast('Error', 'Failed to delete', 'error') }
    finally { isDeletingJail.value = false }
  })
}

// ── SCANNER: PORTS + NMAP ─────────────────────────────────────
const listeningPorts = ref([])
const portFilter = ref('all')
const portSearch = ref('')
const scanTarget = ref('localhost')
const scanStatusMsg = ref('')
const scanResult = ref(null)
const scanResultParsed = ref([])
const isScanning = ref(false)
let scanPollInterval = null
let currentScanJobId = null

const filteredPorts = computed(() => {
  let list = listeningPorts.value
  if (portFilter.value === 'tcp') list = list.filter(p => p.protocol.includes('tcp'))
  if (portFilter.value === 'udp') list = list.filter(p => p.protocol.includes('udp'))
  if (portSearch.value.trim()) {
    const q = portSearch.value.toLowerCase()
    list = list.filter(p =>
      p.port?.toLowerCase().includes(q) ||
      p.process_name?.toLowerCase().includes(q) ||
      p.local_address?.toLowerCase().includes(q) ||
      p.pid?.toLowerCase().includes(q)
    )
  }
  return list
})

const DANGEROUS_PORTS = ['23', '3389', '445', '139', '1433', '3306', '5432', '6379', '27017']
const isDangerousPort = (port) => DANGEROUS_PORTS.includes(String(port))

const fetchPorts = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/ports`)
    listeningPorts.value = await res.json()
  } catch (e) {}
}

const extractPid = (processStr) => {
  if (!processStr) return null
  const match = processStr.match(/pid=(\d+)/)
  return match ? match[1] : null
}

const killPortProcess = (pid) => {
  showConfirm('Kill Process', `Force kill process PID ${pid}?`, async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/process/kill/${pid}`, { method: 'POST' })
      const result = await res.json()
      if (res.ok) { showToast('Success', result.message, 'success'); fetchPorts() }
      else showToast('Error', `Error: ${result}`, 'error')
    } catch (e) { showToast('Error', 'Failed to kill process.', 'error') }
  })
}

const parseNmapResult = (raw) => {
  if (!raw) return []
  const lines = raw.split('\n')
  const results = []
  for (const line of lines) {
    const match = line.match(/^(\d+\/\w+)\s+(\w+)\s+(.*)$/)
    if (match) {
      results.push({ port: match[1], state: match[2], service: match[3].trim() })
    }
  }
  return results
}

const startScan = async () => {
  if (!scanTarget.value) return
  isScanning.value = true
  scanStatusMsg.value = 'Initiating scan...'
  scanResult.value = null
  scanResultParsed.value = []
  if (scanPollInterval) clearInterval(scanPollInterval)
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/ports/scan`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ target: scanTarget.value })
    })
    if (!res.ok) throw new Error(await res.text())
    const data = await res.json()
    currentScanJobId = data.job_id
    scanStatusMsg.value = `Scanning ${scanTarget.value}...`
    scanPollInterval = setInterval(pollScan, 2000)
  } catch (e) {
    scanStatusMsg.value = `Error: ${e.message}`
    isScanning.value = false
  }
}

const pollScan = async () => {
  if (!currentScanJobId) return
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/ports/scan/${currentScanJobId}`)
    const job = await res.json()
    if (job.status === 'done' || job.status === 'failed') {
      clearInterval(scanPollInterval)
      isScanning.value = false
      scanStatusMsg.value = job.status === 'done' ? `Scan complete — ${scanTarget.value}` : 'Scan failed!'
      if (job.status === 'done') {
        try { const parsed = JSON.parse(job.result_json); scanResult.value = parsed.raw_output || job.result_json }
        catch (e) { scanResult.value = job.result_json }
        scanResultParsed.value = parseNmapResult(scanResult.value)
      }
    } else {
      scanStatusMsg.value = `Scanning ${scanTarget.value}...`
    }
  } catch (e) {}
}

// ── LIFECYCLE ─────────────────────────────────────────────────
let pollInterval = null

onMounted(() => {
  fetchNetwork()
  fetchSpeedHistory()
  fetchUfw()
  fetchFail2Ban()
  fetchF2bLogs()
  fetchPorts()
  pollInterval = setInterval(() => {
    fetchNetwork()
    fetchPorts()
  }, 5000)
})

onUnmounted(() => {
  clearInterval(pollInterval)
  if (scanPollInterval) clearInterval(scanPollInterval)
})
</script>

<template>
  <div class="space-y-4">

    <!-- ── TABS HEADER ── -->
    <div class="flex items-center gap-1 border-b" :class="isDark ? 'border-slate-800' : 'border-slate-200'">
      <button v-for="tab in [
        { id: 'network', icon: 'Globe', label: 'Network' },
        { id: 'security', icon: 'ShieldCheck', label: 'Security' },
        { id: 'scanner', icon: 'Radar', label: 'Scanner' }
      ]" :key="tab.id" @click="activeTab = tab.id"
        class="px-5 py-2.5 text-sm font-semibold border-b-2 transition-colors flex items-center gap-2"
        :class="activeTab === tab.id
          ? 'border-brand-500 text-brand-600 dark:text-brand-400'
          : 'border-transparent text-slate-500 hover:text-slate-700 dark:hover:text-slate-300'">
        <Globe v-if="tab.id === 'network'" class="w-4 h-4" />
        <ShieldCheck v-else-if="tab.id === 'security'" class="w-4 h-4" />
        <Radar v-else class="w-4 h-4" />
        {{ tab.label }}
      </button>
    </div>

    <!-- ══ TAB: NETWORK ══ -->
    <div v-if="activeTab === 'network'" class="space-y-6">

      <!-- Network Interfaces -->
      <section class="card">
        <div class="flex items-center justify-between mb-5">
          <h2 class="card-title mb-0"><Network class="w-5 h-5 text-brand-500" /> Network Interfaces</h2>
          <button @click="fetchNetwork" class="p-1.5 rounded-lg text-slate-400 hover:text-brand-500 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors" title="Refresh">
            <RefreshCw class="w-4 h-4" :class="isLoadingNetwork ? 'animate-spin' : ''" />
          </button>
        </div>

        <div v-if="isLoadingNetwork" class="flex justify-center py-10">
          <Loader2 class="w-6 h-6 animate-spin text-brand-500" />
        </div>

        <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
          <div v-for="iface in networkInterfaces" :key="iface.name"
            class="rounded-xl border p-4 space-y-3"
            :class="isDark ? 'border-slate-700 bg-slate-800/30' : 'border-slate-200 bg-slate-50'">
            <!-- Header -->
            <div class="flex items-start justify-between">
              <div>
                <div class="font-bold text-sm" :class="isDark ? 'text-slate-100' : 'text-slate-800'">{{ iface.name }}</div>
                <div class="text-[10px] font-mono text-slate-500 mt-0.5">{{ iface.mac_address }}</div>
              </div>
              <div class="w-2.5 h-2.5 rounded-full mt-1" :class="iface.ip_networks?.length > 0 ? 'bg-emerald-400' : 'bg-slate-300'"></div>
            </div>

            <!-- IPs + Gateway -->
            <div class="space-y-0.5">
              <div v-for="ip in iface.ip_networks" :key="ip" class="text-xs font-mono" :class="isDark ? 'text-slate-300' : 'text-slate-700'">
                {{ ip }}
              </div>
              <div v-if="iface.gateway" class="text-[10px] text-emerald-600 dark:text-emerald-400 font-mono pt-1 border-t mt-1" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
                ⬆ Gateway: {{ iface.gateway }}
              </div>
            </div>

            <!-- RX/TX Bars -->
            <div class="space-y-1.5">
              <div>
                <div class="flex justify-between text-[10px] mb-0.5">
                  <span class="text-slate-500">⬇ Received</span>
                  <span class="font-mono text-green-600 dark:text-green-400">{{ formatBytes(iface.rx_bytes) }}</span>
                </div>
                <div class="h-1.5 rounded-full overflow-hidden" :class="isDark ? 'bg-slate-700' : 'bg-slate-200'">
                  <div class="h-1.5 bg-green-500 rounded-full" :style="`width: ${Math.min((iface.rx_bytes / Math.max(iface.rx_bytes, iface.tx_bytes, 1)) * 100, 100)}%`"></div>
                </div>
              </div>
              <div>
                <div class="flex justify-between text-[10px] mb-0.5">
                  <span class="text-slate-500">⬆ Transmitted</span>
                  <span class="font-mono text-blue-600 dark:text-blue-400">{{ formatBytes(iface.tx_bytes) }}</span>
                </div>
                <div class="h-1.5 rounded-full overflow-hidden" :class="isDark ? 'bg-slate-700' : 'bg-slate-200'">
                  <div class="h-1.5 bg-blue-500 rounded-full" :style="`width: ${Math.min((iface.tx_bytes / Math.max(iface.rx_bytes, iface.tx_bytes, 1)) * 100, 100)}%`"></div>
                </div>
              </div>
            </div>
          </div>

          <div v-if="networkInterfaces.length === 0" class="col-span-full text-center py-10 text-slate-400">
            <Network class="w-8 h-8 mx-auto mb-2 opacity-30" />
            <p class="text-sm">No network interfaces detected</p>
          </div>
        </div>
      </section>

      <!-- Speedtest -->
      <section class="card">
        <div class="flex items-center justify-between mb-5">
          <h2 class="card-title mb-0"><Activity class="w-5 h-5 text-brand-500" /> Internet Speedtest</h2>
          <button @click="runSpeedtest" class="btn-primary" :disabled="isRunningSpeedtest">
            <Loader2 v-if="isRunningSpeedtest" class="w-4 h-4 animate-spin" />
            <Play v-else class="w-4 h-4" />
            {{ isRunningSpeedtest ? 'Testing...' : 'Run Speedtest' }}
          </button>
        </div>

        <!-- Last Result Summary -->
        <div v-if="latestSpeed" class="grid grid-cols-3 gap-4 mb-6 p-4 rounded-xl" :class="isDark ? 'bg-slate-800/50' : 'bg-slate-50'">
          <div class="text-center">
            <div class="text-2xl font-bold text-green-600 dark:text-green-400">{{ formatMbps(latestSpeed.download_mbps) }}</div>
            <div class="text-xs text-slate-500 mt-0.5 flex items-center justify-center gap-1"><Download class="w-3 h-3" /> Download</div>
          </div>
          <div class="text-center border-x" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
            <div class="text-2xl font-bold text-blue-600 dark:text-blue-400">{{ formatMbps(latestSpeed.upload_mbps) }}</div>
            <div class="text-xs text-slate-500 mt-0.5 flex items-center justify-center gap-1"><Upload class="w-3 h-3" /> Upload</div>
          </div>
          <div class="text-center">
            <div class="text-2xl font-bold text-amber-600 dark:text-amber-400">{{ latestSpeed.ping_ms ? latestSpeed.ping_ms.toFixed(0) + ' ms' : '—' }}</div>
            <div class="text-xs text-slate-500 mt-0.5">Ping</div>
          </div>
        </div>

        <!-- History Table -->
        <div class="overflow-x-auto rounded-xl border" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
          <table class="w-full">
            <thead class="text-xs" :class="isDark ? 'bg-slate-800/50 border-b border-slate-700 text-slate-400' : 'bg-slate-50 border-b border-slate-200 text-slate-500'">
              <tr>
                <th class="px-4 py-3 text-left font-semibold">Date & Time</th>
                <th class="px-4 py-3 text-left font-semibold">Download</th>
                <th class="px-4 py-3 text-left font-semibold">Upload</th>
                <th class="px-4 py-3 text-left font-semibold">Ping</th>
                <th class="px-4 py-3 text-left font-semibold">Server</th>
              </tr>
            </thead>
            <tbody class="divide-y" :class="isDark ? 'divide-slate-800' : 'divide-slate-100'">
              <tr v-if="isLoadingSpeedtest">
                <td colspan="5" class="text-center py-8 text-slate-400 text-sm">Loading history...</td>
              </tr>
              <tr v-else-if="speedHistory.length === 0">
                <td colspan="5" class="text-center py-8 text-slate-400 text-sm">No speedtest history found. Run a test to begin.</td>
              </tr>
              <tr v-else v-for="item in speedHistory" :key="item.id" class="transition-colors" :class="isDark ? 'hover:bg-slate-800/40' : 'hover:bg-slate-50'">
                <td class="px-4 py-3 text-xs text-slate-500">{{ formatDate(item.tested_at) }}</td>
                <td class="px-4 py-3 font-semibold text-green-600 dark:text-green-400 flex items-center gap-1.5"><Download class="w-3.5 h-3.5" />{{ formatMbps(item.download_mbps) }}</td>
                <td class="px-4 py-3 font-semibold text-blue-600 dark:text-blue-400"><Upload class="w-3.5 h-3.5 inline mr-1.5" />{{ formatMbps(item.upload_mbps) }}</td>
                <td class="px-4 py-3 text-amber-600 dark:text-amber-400 font-medium">{{ item.ping_ms ? item.ping_ms.toFixed(1) + ' ms' : '—' }}</td>
                <td class="px-4 py-3 text-xs text-slate-500 truncate max-w-[150px]" :title="item.server_name">{{ item.server_name || 'Unknown' }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

    </div>

    <!-- ══ TAB: SECURITY ══ -->
    <div v-if="activeTab === 'security'" class="grid grid-cols-1 xl:grid-cols-2 gap-6">

      <!-- ── UFW FIREWALL ── -->
      <section class="card flex flex-col">
        <div class="flex items-center justify-between mb-5">
          <h2 class="card-title mb-0"><ShieldCheck class="w-5 h-5 text-brand-500" /> UFW Firewall</h2>
          <button @click="fetchUfw" class="p-1.5 rounded-lg text-slate-400 hover:text-brand-500 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors">
            <RefreshCw class="w-3.5 h-3.5" :class="isLoadingUfw ? 'animate-spin' : ''" />
          </button>
        </div>

        <div v-if="isLoadingUfw" class="flex justify-center py-8"><Loader2 class="w-5 h-5 animate-spin text-brand-500" /></div>

        <template v-else-if="ufwStatus">
          <!-- Status Toggle -->
          <div class="flex items-center justify-between p-4 rounded-xl mb-4 border-2 transition-colors"
            :class="ufwStatus.enabled
              ? (isDark ? 'bg-emerald-900/10 border-emerald-500/30' : 'bg-emerald-50 border-emerald-200')
              : (isDark ? 'bg-slate-800/50 border-slate-700' : 'bg-slate-50 border-slate-200')">
            <div>
              <div class="font-bold text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-700'">Firewall Status</div>
              <div class="text-xs mt-0.5" :class="ufwStatus.enabled ? 'text-emerald-600 dark:text-emerald-400' : 'text-slate-500'">
                {{ ufwStatus.enabled ? 'Active — protecting your server' : 'Inactive — server is unprotected' }}
              </div>
            </div>
            <button @click="toggleUfw" :disabled="isTogglingUfw"
              class="flex items-center gap-2 px-4 py-2 rounded-lg font-semibold text-sm transition-colors"
              :class="ufwStatus.enabled
                ? 'bg-red-100 text-red-600 hover:bg-red-200 dark:bg-red-900/20 dark:text-red-400 dark:hover:bg-red-900/40'
                : 'bg-emerald-100 text-emerald-700 hover:bg-emerald-200 dark:bg-emerald-900/20 dark:text-emerald-400 dark:hover:bg-emerald-900/40'">
              <Loader2 v-if="isTogglingUfw" class="w-4 h-4 animate-spin" />
              <ToggleRight v-else-if="ufwStatus.enabled" class="w-4 h-4" />
              <ToggleLeft v-else class="w-4 h-4" />
              {{ isTogglingUfw ? '...' : (ufwStatus.enabled ? 'Disable' : 'Enable') }}
            </button>
          </div>

          <!-- Add Rule Form -->
          <div class="rounded-xl border p-4 mb-4" :class="isDark ? 'border-slate-700 bg-slate-800/30' : 'border-slate-200 bg-slate-50'">
            <div class="text-xs font-semibold mb-3" :class="isDark ? 'text-slate-300' : 'text-slate-600'">Add Firewall Rule</div>
            <div class="grid grid-cols-[110px_1fr] gap-3 mb-3">
              <!-- Action -->
              <div>
                <label class="block text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 mb-1">Action</label>
                <select v-model="newRule.action" class="input-field text-sm">
                  <option value="allow">Allow</option>
                  <option value="deny">Deny</option>
                </select>
              </div>
              <!-- Port / Service -->
              <div>
                <label class="block text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 mb-1">Port / Service</label>
                <input v-model="newRule.port" type="text"
                  placeholder="e.g. 80, 443, 22, ssh"
                  class="input-field text-sm"
                  @keyup.enter="addUfwRule"
                  :disabled="isAddingRule" />
                <p class="text-[10px] text-slate-400 dark:text-slate-500 mt-1">
                  Port number, range (80:90/tcp), atau nama service
                </p>
              </div>
            </div>
            <div class="flex justify-end">
              <button @click="addUfwRule" class="btn-primary text-sm" :disabled="isAddingRule">
                <Loader2 v-if="isAddingRule" class="w-4 h-4 animate-spin" />
                <Plus v-else class="w-4 h-4" />
                Add Rule
              </button>
            </div>
          </div>

          <!-- Rules Table -->
          <div class="flex-1 overflow-auto rounded-xl border" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
            <table class="w-full">
              <thead class="sticky top-0 text-xs" :class="isDark ? 'bg-slate-800 border-b border-slate-700 text-slate-400' : 'bg-slate-50 border-b border-slate-200 text-slate-500'">
                <tr>
                  <th class="px-3 py-2 text-left font-semibold">Rule</th>
                  <th class="px-3 py-2 text-right w-16 font-semibold">Action</th>
                </tr>
              </thead>
              <tbody class="divide-y text-sm" :class="isDark ? 'divide-slate-800' : 'divide-slate-100'">
                <tr v-if="ufwStatus.rules.length === 0">
                  <td colspan="2" class="text-center py-6 text-slate-400 text-xs italic">No firewall rules configured.</td>
                </tr>
                <tr v-else v-for="rule in ufwStatus.rules" :key="rule" class="group" :class="isDark ? 'hover:bg-slate-800/40' : 'hover:bg-slate-50'">
                  <td class="px-3 py-2 font-mono text-xs" :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ rule }}</td>
                  <td class="px-3 py-2 text-right">
                    <button @click="deleteUfwRule(rule.split(' ')[0])"
                      class="p-1 rounded opacity-0 group-hover:opacity-100 transition-opacity text-red-400 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20">
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>
      </section>

      <!-- ── FAIL2BAN ── -->
      <section class="card flex flex-col gap-4">
        <div class="flex items-center justify-between">
          <h2 class="card-title mb-0"><Ban class="w-5 h-5 text-brand-500" /> Intrusion Prevention</h2>
          <button v-if="f2bStatus?.installed" @click="fetchFail2Ban" class="p-1.5 rounded-lg text-slate-400 hover:text-brand-500 hover:bg-slate-100 dark:hover:bg-slate-700">
            <RefreshCw class="w-3.5 h-3.5" />
          </button>
        </div>

        <div v-if="isLoadingF2b" class="flex justify-center py-8"><Loader2 class="w-5 h-5 animate-spin text-brand-500" /></div>

        <!-- Not Installed -->
        <div v-else-if="f2bStatus && !f2bStatus.installed" class="text-center py-8 space-y-4">
          <div class="w-14 h-14 rounded-2xl bg-slate-100 dark:bg-slate-800 flex items-center justify-center mx-auto">
            <Ban class="w-7 h-7 text-slate-400" />
          </div>
          <div>
            <p class="text-sm font-semibold" :class="isDark ? 'text-slate-200' : 'text-slate-700'">Fail2Ban not installed</p>
            <p class="text-xs text-slate-500 mt-1">Protect SSH and other services from brute-force attacks</p>
          </div>
          <button @click="installFail2Ban" class="btn-primary mx-auto" :disabled="isInstallingF2b">
            <Loader2 v-if="isInstallingF2b" class="w-4 h-4 animate-spin" />
            <Plus v-else class="w-4 h-4" />
            {{ isInstallingF2b ? 'Installing...' : 'Install Fail2Ban' }}
          </button>
        </div>

        <template v-else-if="f2bStatus?.installed">
          <!-- Status Row -->
          <div class="grid grid-cols-3 gap-3">
            <div class="rounded-xl border p-3 text-center" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
              <div class="flex items-center justify-center gap-1.5 mb-0.5">
                <div class="w-2 h-2 rounded-full" :class="f2bStatus.active ? 'bg-emerald-400 animate-pulse' : 'bg-slate-400'"></div>
                <span class="text-xs font-bold" :class="f2bStatus.active ? 'text-emerald-500' : 'text-slate-400'">{{ f2bStatus.active ? 'Active' : 'Inactive' }}</span>
              </div>
              <div class="text-[10px] text-slate-500">Service</div>
            </div>
            <div class="rounded-xl border p-3 text-center" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
              <div class="text-lg font-bold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">{{ f2bStatus.jails?.length || 0 }}</div>
              <div class="text-[10px] text-slate-500">Active Jails</div>
            </div>
            <div class="rounded-xl border p-3 text-center" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
              <div class="text-lg font-bold text-red-500">{{ totalBanned }}</div>
              <div class="text-[10px] text-slate-500">Banned IPs</div>
            </div>
          </div>

          <!-- Manual Ban Form -->
          <div class="rounded-xl border p-3" :class="isDark ? 'border-slate-700 bg-slate-800/30' : 'border-slate-200 bg-slate-50'">
            <div class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-2">Manual Ban IP</div>
            <div class="grid grid-cols-[110px_1fr_auto] gap-2">
              <select v-model="formManualBan.jail" class="input-field py-1.5 text-xs">
                <option v-for="j in f2bStatus.jails" :key="j.name" :value="j.name">{{ j.name }}</option>
              </select>
              <input v-model="formManualBan.ip" type="text" placeholder="IP to ban..." class="input-field py-1.5 text-xs" :disabled="isBanning" @keyup.enter="banIp" />
              <button @click="banIp" class="btn-danger py-1.5 px-3 text-xs shrink-0" :disabled="isBanning">
                <Loader2 v-if="isBanning" class="w-3.5 h-3.5 animate-spin" />
                <Ban v-else class="w-3.5 h-3.5" />
                Ban
              </button>
            </div>
          </div>

          <!-- Jails List -->
          <div class="space-y-2 max-h-48 overflow-y-auto">
            <div v-for="jail in f2bStatus.jails" :key="jail.name" class="rounded-xl border overflow-hidden" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
              <div class="px-3 py-2 flex justify-between items-center" :class="isDark ? 'bg-slate-800/60' : 'bg-slate-50'">
                <span class="font-bold text-xs" :class="isDark ? 'text-slate-200' : 'text-slate-700'">{{ jail.name }}</span>
                <span class="text-[10px] px-2 py-0.5 rounded-full font-semibold" :class="jail.banned_ips.length > 0 ? 'bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400' : 'bg-slate-100 text-slate-500 dark:bg-slate-700 dark:text-slate-400'">
                  {{ jail.banned_ips.length }} Banned
                </span>
              </div>
              <div v-if="jail.banned_ips.length > 0" class="p-2 flex flex-wrap gap-1.5">
                <div v-for="ip in jail.banned_ips" :key="ip"
                  class="flex items-center gap-1.5 px-2 py-1 rounded-lg border text-xs font-mono"
                  :class="isDark ? 'bg-slate-900 border-slate-700 text-slate-300' : 'bg-white border-slate-200 text-slate-700'">
                  {{ ip }}
                  <button @click="unbanIp(jail.name, ip)" class="text-emerald-500 hover:text-emerald-600 font-bold text-[10px] uppercase">Unban</button>
                </div>
              </div>
            </div>
            <div v-if="f2bStatus.jails?.length === 0" class="text-center py-4 text-xs text-slate-400 italic border border-dashed rounded-xl">
              No active jails. Click Configure Jails to enable protection.
            </div>
          </div>

          <!-- Mini Tabs: Logs + Configure -->
          <div>
            <div class="flex items-center gap-3 mb-2">
              <button @click="f2bActiveTab = 'logs'" class="text-xs font-semibold border-b-2 pb-1 transition-colors"
                :class="f2bActiveTab === 'logs' ? 'border-brand-500 text-brand-500' : 'border-transparent text-slate-500 hover:text-slate-700'">
                Live Logs
              </button>
              <button @click="openConfigModal" class="ml-auto btn-outline text-xs py-1 px-3">
                <ShieldCheck class="w-3.5 h-3.5" /> Configure Jails
              </button>
            </div>
            <div class="bg-black/90 text-slate-300 font-mono text-[10px] p-3 rounded-xl overflow-y-auto h-[180px] leading-relaxed">
              <div v-if="f2bLogs.length === 0" class="text-slate-600 italic">No recent logs...</div>
              <div v-for="(line, idx) in f2bLogs" :key="idx" class="py-0.5 hover:bg-white/5 px-1"
                :class="{'text-red-400': line.includes('Ban'), 'text-emerald-400': line.includes('Unban'), 'text-blue-400': line.includes('Found')}">
                {{ line }}
              </div>
            </div>
          </div>

        </template>
      </section>
    </div>

    <!-- ══ TAB: SCANNER ══ -->
    <div v-if="activeTab === 'scanner'" class="space-y-6">

      <!-- Nmap Scanner -->
      <section class="card">
        <h2 class="card-title"><Radar class="w-5 h-5 text-brand-500" /> Deep Port Scan (Nmap)</h2>
        <div class="flex gap-3 mt-4 mb-4">
          <input v-model="scanTarget" type="text" placeholder="Target IP or hostname (e.g. 192.168.1.1)" class="input-field flex-1" :disabled="isScanning" @keyup.enter="startScan" />
          <button @click="startScan" class="btn-primary shrink-0" :disabled="isScanning">
            <Loader2 v-if="isScanning" class="w-4 h-4 animate-spin" />
            <Radar v-else class="w-4 h-4" />
            {{ isScanning ? 'Scanning...' : 'Run Scan' }}
          </button>
        </div>

        <!-- Scan status -->
        <div v-if="scanStatusMsg" class="flex items-center gap-2 text-sm mb-4 p-3 rounded-lg"
          :class="scanResultParsed.length > 0 ? (isDark ? 'bg-emerald-900/10 text-emerald-400' : 'bg-emerald-50 text-emerald-700') : (isDark ? 'bg-slate-800/50 text-slate-400' : 'bg-slate-50 text-slate-600')">
          <Loader2 v-if="isScanning" class="w-4 h-4 animate-spin shrink-0" />
          <CheckCircle2 v-else-if="scanResultParsed.length > 0" class="w-4 h-4 shrink-0" />
          {{ scanStatusMsg }}
          <span v-if="scanResultParsed.length > 0" class="ml-auto font-semibold text-xs">{{ scanResultParsed.length }} port(s) found</span>
        </div>

        <!-- Scan Results Table -->
        <div v-if="scanResultParsed.length > 0" class="rounded-xl border overflow-hidden" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
          <table class="w-full">
            <thead class="text-xs" :class="isDark ? 'bg-slate-800/50 border-b border-slate-700 text-slate-400' : 'bg-slate-50 border-b border-slate-200 text-slate-500'">
              <tr>
                <th class="px-4 py-2.5 text-left font-semibold w-28">Port/Proto</th>
                <th class="px-4 py-2.5 text-left font-semibold w-20">State</th>
                <th class="px-4 py-2.5 text-left font-semibold">Service</th>
                <th class="px-4 py-2.5 text-left font-semibold w-20">Risk</th>
              </tr>
            </thead>
            <tbody class="divide-y text-sm" :class="isDark ? 'divide-slate-800' : 'divide-slate-100'">
              <tr v-for="row in scanResultParsed" :key="row.port"
                class="transition-colors" :class="[isDark ? 'hover:bg-slate-800/40' : 'hover:bg-slate-50', isDangerousPort(row.port) ? (isDark ? 'bg-red-900/5' : 'bg-red-50/50') : '']">
                <td class="px-4 py-2.5 font-mono text-xs font-bold" :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ row.port }}</td>
                <td class="px-4 py-2.5">
                  <span class="px-2 py-0.5 rounded text-[10px] font-semibold"
                    :class="row.state === 'open' ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400' : 'bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-400'">
                    {{ row.state }}
                  </span>
                </td>
                <td class="px-4 py-2.5 text-xs" :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ row.service || '—' }}</td>
                <td class="px-4 py-2.5">
                  <span v-if="isDangerousPort(row.port)" class="flex items-center gap-1 text-[10px] font-semibold text-red-500">
                    <AlertTriangle class="w-3 h-3" /> High Risk
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-else-if="!isScanning && !scanStatusMsg" class="text-center py-12 text-slate-400">
          <Radar class="w-10 h-10 mx-auto mb-3 opacity-30" />
          <p class="text-sm">Enter a target and click Run Scan to begin</p>
        </div>
      </section>

      <!-- Listening Ports -->
      <section class="card">
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-4">
          <h2 class="card-title mb-0"><Network class="w-5 h-5 text-brand-500" /> Listening Ports</h2>
          <div class="flex items-center gap-2">
            <!-- Protocol Filter -->
            <div class="flex rounded-lg border overflow-hidden text-xs" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
              <button v-for="f in ['all', 'tcp', 'udp']" :key="f" @click="portFilter = f"
                class="px-3 py-1.5 font-semibold transition-colors capitalize"
                :class="portFilter === f
                  ? 'bg-brand-500 text-white'
                  : (isDark ? 'bg-slate-800 text-slate-400 hover:bg-slate-700' : 'bg-white text-slate-600 hover:bg-slate-50')">
                {{ f }}
              </button>
            </div>
            <!-- Search -->
            <div class="relative">
              <Search class="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400" />
              <input v-model="portSearch" type="text" placeholder="Search..." class="input-field py-1.5 pl-8 text-xs w-40" />
            </div>
          </div>
        </div>

        <div class="rounded-xl border overflow-hidden" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
          <table class="w-full">
            <thead class="sticky top-0 text-xs" :class="isDark ? 'bg-slate-800/80 border-b border-slate-700 text-slate-400' : 'bg-slate-50 border-b border-slate-200 text-slate-500'">
              <tr>
                <th class="px-4 py-2.5 text-left font-semibold w-16">Port</th>
                <th class="px-4 py-2.5 text-left font-semibold w-16">Proto</th>
                <th class="px-4 py-2.5 text-left font-semibold w-24">Scope</th>
                <th class="px-4 py-2.5 text-left font-semibold">Process</th>
                <th class="px-4 py-2.5 text-left font-semibold w-20">PID</th>
                <th class="px-4 py-2.5 text-right w-20 font-semibold">Action</th>
              </tr>
            </thead>
            <tbody class="divide-y text-sm" :class="isDark ? 'divide-slate-800' : 'divide-slate-100'">
              <tr v-for="port in filteredPorts" :key="port.local_address + port.protocol"
                class="transition-colors group"
                :class="[isDark ? 'hover:bg-slate-800/40' : 'hover:bg-slate-50', isDangerousPort(port.port) ? (isDark ? 'bg-red-900/5' : 'bg-red-50/30') : '']">
                <!-- Port number -->
                <td class="px-4 py-2.5">
                  <div class="flex items-center gap-1.5">
                    <span class="font-mono font-bold text-xs"
                      :class="isDangerousPort(port.port) ? 'text-red-600 dark:text-red-400' : (isDark ? 'text-slate-200' : 'text-slate-800')">
                      {{ port.port }}
                    </span>
                    <AlertTriangle v-if="isDangerousPort(port.port)" class="w-3 h-3 text-red-400 shrink-0" />
                  </div>
                  <!-- full address on hover -->
                  <div class="text-[9px] text-slate-400 font-mono truncate max-w-[80px]" :title="port.local_address">{{ port.local_address }}</div>
                </td>
                <!-- Protocol -->
                <td class="px-4 py-2.5">
                  <span class="px-1.5 py-0.5 rounded text-[10px] font-semibold"
                    :class="port.protocol.includes('tcp') ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300' : 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300'">
                    {{ port.protocol.toUpperCase().replace('V6', '') }}
                  </span>
                </td>
                <!-- Scope badge -->
                <td class="px-4 py-2.5">
                  <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold"
                    :class="port.scope === 'public'
                      ? 'bg-orange-100 text-orange-700 dark:bg-orange-900/25 dark:text-orange-400'
                      : 'bg-slate-100 text-slate-600 dark:bg-slate-700 dark:text-slate-400'">
                    <span>{{ port.scope === 'public' ? '🌍' : '🔒' }}</span>
                    {{ port.scope === 'public' ? 'Public' : 'Local' }}
                  </span>
                </td>
                <!-- Process Name -->
                <td class="px-4 py-2.5 text-xs" :class="isDark ? 'text-slate-300' : 'text-slate-700'">
                  {{ port.process_name || '—' }}
                </td>
                <!-- PID -->
                <td class="px-4 py-2.5">
                  <span v-if="port.pid" class="font-mono text-[10px] px-1.5 py-0.5 rounded"
                    :class="isDark ? 'bg-slate-700 text-slate-400' : 'bg-slate-100 text-slate-500'">
                    {{ port.pid }}
                  </span>
                </td>
                <!-- Action: Kill -->
                <td class="px-4 py-2.5 text-right">
                  <button v-if="port.pid" @click="killPortProcess(port.pid)"
                    class="p-1.5 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity text-red-400 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                    title="Kill process">
                    <XCircle class="w-3.5 h-3.5" />
                  </button>
                </td>
              </tr>
              <tr v-if="filteredPorts.length === 0">
                <td colspan="6" class="text-center py-8 text-slate-400 text-sm">
                  <Network class="w-6 h-6 mx-auto mb-2 opacity-30" />
                  No ports match the current filter
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="text-xs text-slate-400 mt-2 px-1">{{ filteredPorts.length }} of {{ listeningPorts.length }} ports shown</div>
      </section>

    </div>

    <!-- ── JAIL CONFIG MODAL ── -->
    <Teleport to="body">
      <div v-if="showConfigModal" class="fixed inset-0 z-[100] backdrop-blur-sm flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
        <div class="rounded-xl shadow-2xl w-full max-w-5xl overflow-hidden flex flex-col max-h-[90vh]" :class="isDark ? 'bg-slate-800' : 'bg-white'">
          <div class="p-4 border-b flex justify-between items-center shrink-0" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'">
            <h3 class="font-bold flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'">
              <ShieldCheck class="w-4 h-4 text-brand-500" /> Jail Configuration — /etc/fail2ban/jail.local
            </h3>
            <button @click="showConfigModal = false" class="text-slate-400 hover:text-slate-200 text-lg leading-none">✕</button>
          </div>

          <div class="flex-1 overflow-hidden flex flex-col md:flex-row min-h-0">
            <!-- Sidebar -->
            <div class="w-full md:w-72 border-r overflow-y-auto shrink-0 flex flex-col" :class="isDark ? 'border-slate-700 bg-slate-900/30' : 'border-slate-200 bg-slate-50'">
              <div class="p-3 border-b" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
                <div class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-2">Quick Add Service</div>
                <div class="space-y-1">
                  <button v-for="t in SERVICE_TEMPLATES" :key="t.name" @click="applyTemplate(t)"
                    class="w-full flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs transition-colors text-left"
                    :class="isDark ? 'hover:bg-slate-700 text-slate-300' : 'hover:bg-white text-slate-700'">
                    <span class="text-base leading-none">{{ t.icon }}</span>
                    <div>
                      <div class="font-semibold">{{ t.label }}</div>
                      <div class="text-[10px] text-slate-500">{{ t.name }}</div>
                    </div>
                  </button>
                </div>
              </div>
              <div class="p-3 flex-1">
                <div class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-2">Configured Jails</div>
                <button @click="createNewJail" class="w-full btn-outline border-dashed text-xs py-1.5 mb-2">+ Create Custom Jail</button>
                <div v-for="j in f2bConfig" :key="j.name" @click="editJail(j)"
                  class="p-2.5 border rounded-lg cursor-pointer transition-colors flex justify-between items-center mb-1"
                  :class="[formConfig.name === j.name ? (isDark ? 'bg-brand-900/30 border-brand-500' : 'bg-brand-50 border-brand-400') : (isDark ? 'border-slate-700 hover:bg-slate-700' : 'border-slate-200 hover:bg-white'), !j.enabled ? 'opacity-50' : '']">
                  <div>
                    <div class="font-bold text-xs" :class="isDark ? 'text-slate-200' : 'text-slate-700'">{{ j.name }}</div>
                    <div class="text-[10px] text-slate-500">{{ j.port || 'Any port' }}</div>
                  </div>
                  <div class="w-2 h-2 rounded-full shrink-0" :class="j.enabled ? 'bg-green-500' : 'bg-slate-400'"></div>
                </div>
              </div>
            </div>

            <!-- Editor -->
            <div class="flex-1 overflow-y-auto flex flex-col min-h-0">
              <div v-if="!formConfig.name" class="flex-1 flex flex-col items-center justify-center text-slate-500 text-sm gap-3 p-6">
                <ShieldCheck class="w-10 h-10 opacity-30" />
                <p>Select a jail or click a Quick Add template to get started.</p>
              </div>
              <div v-else class="p-6 space-y-5 flex-1">
                <div class="flex justify-between items-start gap-4">
                  <div>
                    <h3 class="text-base font-bold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">{{ formConfig.name === 'DEFAULT' ? 'Global Default Settings' : `Editing: ${formConfig.name}` }}</h3>
                    <p class="text-[10px] text-slate-500 mt-0.5">Changes are saved to /etc/fail2ban/jail.local</p>
                  </div>
                  <label class="flex items-center gap-2 cursor-pointer shrink-0" v-if="formConfig.name !== 'DEFAULT'">
                    <span class="text-sm font-semibold">Enabled</span>
                    <input type="checkbox" v-model="formConfig.enabled" class="rounded text-brand-600 focus:ring-brand-500" />
                  </label>
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <div v-if="formConfig.name !== 'DEFAULT'">
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Jail Name</label>
                    <input v-model="formConfig.name" type="text" class="input-field w-full text-sm" :disabled="f2bConfig.find(x => x.name === formConfig.name) && formConfig.name !== 'custom-'" />
                  </div>
                  <div v-if="formConfig.name !== 'DEFAULT'">
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Port <span class="text-[10px] text-slate-400">(e.g. ssh, http, 8080)</span></label>
                    <input v-model="formConfig.port" type="text" placeholder="ssh" class="input-field w-full text-sm" />
                  </div>
                  <div>
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Max Retry <span class="text-[10px] text-slate-400 font-normal">Percobaan gagal sebelum ban</span></label>
                    <input v-model="formConfig.maxretry" type="text" placeholder="5" class="input-field w-full text-sm" />
                  </div>
                  <div>
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Find Time <span class="text-[10px] text-slate-400 font-normal">Periode (10m, 1h)</span></label>
                    <input v-model="formConfig.findtime" type="text" placeholder="10m" class="input-field w-full text-sm" />
                  </div>
                  <div>
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Ban Time <span class="text-[10px] text-slate-400 font-normal">Durasi (-1 = permanen)</span></label>
                    <input v-model="formConfig.bantime" type="text" placeholder="1h" class="input-field w-full text-sm" />
                  </div>
                  <div v-if="formConfig.name !== 'DEFAULT'">
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Filter <span class="text-[10px] text-slate-400 font-normal">dari /etc/fail2ban/filter.d/</span></label>
                    <input v-model="formConfig.filter" list="filter-list" type="text" placeholder="sshd" class="input-field w-full text-sm font-mono" />
                    <datalist id="filter-list"><option v-for="f in f2bFilters" :key="f" :value="f" /></datalist>
                  </div>
                  <div class="sm:col-span-2" v-if="formConfig.name !== 'DEFAULT'">
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Log Path <span class="text-[10px] text-slate-400 font-normal">File log yang dipantau</span></label>
                    <input v-model="formConfig.logpath" type="text" placeholder="/var/log/auth.log" class="input-field w-full text-sm font-mono" />
                  </div>
                </div>
                <div v-if="configPreview">
                  <div class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-1.5">Live Preview</div>
                  <pre class="bg-black/80 text-green-400 font-mono text-[11px] p-4 rounded-lg overflow-x-auto leading-relaxed">{{ configPreview }}</pre>
                </div>
                <div class="pt-2 flex justify-between items-center border-t gap-3" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
                  <button v-if="formConfig.name && f2bConfig.find(x => x.name === formConfig.name)" @click="deleteJail" class="btn-danger text-xs" :disabled="isDeletingJail">
                    <Trash2 class="w-3.5 h-3.5" /> {{ isDeletingJail ? 'Deleting...' : 'Delete Jail' }}
                  </button>
                  <div v-else></div>
                  <button @click="saveJailConfig" class="btn-primary">Save & Reload Fail2Ban</button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

  </div>
</template>
