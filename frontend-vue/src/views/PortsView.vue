<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { Network, Radar, ShieldCheck, Activity, Play, History, Download, Upload, Server, CheckCircle2, XCircle, AlertTriangle, Plus } from 'lucide-vue-next'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast, showConfirm } = useToastStore()
const { isDark } = useThemeStore()

// ── NETWORK & PORTS STATE ──
const networkInterfaces = ref([])
const listeningPorts = ref([])
const scanTarget = ref('localhost')
const scanStatusMsg = ref('')
const scanResult = ref(null)
const isScanning = ref(false)

let pollInterval = null
let scanPollInterval = null
let currentScanJobId = null

const fetchNetwork = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/network`)
    networkInterfaces.value = await res.json()
  } catch (e) {}
}

const fetchPorts = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/ports`)
    listeningPorts.value = await res.json()
  } catch (e) {}
}

const startScan = async () => {
  if (!scanTarget.value) return
  isScanning.value = true
  scanStatusMsg.value = 'Initiating scan...'
  scanResult.value = null
  
  if (scanPollInterval) clearInterval(scanPollInterval)

  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/ports/scan`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ target: scanTarget.value })
    })
    
    if(!res.ok) throw new Error(await res.text())
    
    const data = await res.json()
    currentScanJobId = data.job_id
    scanStatusMsg.value = `Job #${currentScanJobId} started. Scanning...`
    
    scanPollInterval = setInterval(pollScan, 2000)
  } catch (e) {
    scanStatusMsg.value = `Error: ${e.message}`
    isScanning.value = false
  }
}

const extractPid = (processStr) => {
  if (!processStr) return null;
  const match = processStr.match(/pid=(\d+)/);
  return match ? match[1] : null;
}

const killPortProcess = (pid) => {
  showConfirm("Konfirmasi", `Are you sure you want to FORCE KILL process PID ${pid}?`, async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/process/kill/${pid}`, { method: 'POST' })
      const result = await res.json()
      if(res.ok) {
        showToast("Success", result.message, "success")
        fetchPorts()
      } else {
        showToast("Error", `Error: ${result}`, "error")
      }
    } catch(e) {
      showToast("Error", "Failed to kill process.", "error")
    }
  })
}

const pollScan = async () => {
  if (!currentScanJobId) return
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/ports/scan/${currentScanJobId}`)
    const job = await res.json()
    
    if (job.status === 'done' || job.status === 'failed') {
      clearInterval(scanPollInterval)
      isScanning.value = false
      scanStatusMsg.value = job.status === 'done' ? 'Scan complete!' : 'Scan failed!'
      
      try {
        const parsed = JSON.parse(job.result_json)
        scanResult.value = parsed.raw_output || job.result_json
      } catch (e) {
        scanResult.value = job.result_json
      }
    } else {
      scanStatusMsg.value = `Job #${currentScanJobId} status: ${job.status}...`
    }
  } catch (e) {}
}

// ── FAIL2BAN STATE ──
const f2bStatus = ref(null)
const f2bLogs = ref([])
const f2bConfig = ref([])
const isLoadingF2b = ref(true)
const isInstallingF2b = ref(false)

const formManualBan = ref({ jail: 'sshd', ip: '' })
const isBanning = ref(false)

const showConfigModal = ref(false)
const formConfig = ref({
  name: '', enabled: false, port: '', logpath: '', filter: '', maxretry: '', bantime: '', findtime: ''
})

const fetchFail2Ban = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/status`)
    if (res.ok) {
      f2bStatus.value = await res.json()
      if (f2bStatus.value.installed) {
        fetchF2bLogs()
      }
    }
  } catch (e) {
  } finally {
    isLoadingF2b.value = false
  }
}

const fetchF2bLogs = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/logs`)
    if (res.ok) f2bLogs.value = await res.json()
  } catch (e) {}
}

const fetchF2bConfig = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/config`)
    if (res.ok) f2bConfig.value = await res.json()
  } catch (e) {}
}

const openConfigModal = async () => {
  await fetchF2bConfig()
  showConfigModal.value = true
}

const editJail = (jail) => {
  formConfig.value = { ...jail }
}

const createNewJail = () => {
  formConfig.value = {
    name: 'custom-', enabled: true, port: '', logpath: '', filter: '', maxretry: '5', bantime: '10m', findtime: '10m'
  }
}

const saveJailConfig = async () => {
  if (!formConfig.value.name) return
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/config`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(formConfig.value)
    })
    if (res.ok) {
      showToast("Success", "Jail configuration saved and reloaded", "success")
      await fetchF2bConfig()
      await fetchFail2Ban() // refresh status
      formConfig.value = { name: '', enabled: false, port: '', logpath: '', filter: '', maxretry: '', bantime: '', findtime: '' }
    } else {
      showToast("Error", await res.text(), "error")
    }
  } catch (e) {
    showToast("Error", "Failed to save configuration", "error")
  }
}

const banIp = async () => {
  if (!formManualBan.value.ip || !formManualBan.value.jail) return
  isBanning.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/ban`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(formManualBan.value)
    })
    if (res.ok) {
      showToast("Success", "IP has been banned", "success")
      formManualBan.value.ip = ''
      await fetchFail2Ban()
    } else {
      showToast("Error", await res.text(), "error")
    }
  } catch (e) {
    showToast("Error", "Failed to ban IP", "error")
  } finally {
    isBanning.value = false
  }
}

const installFail2Ban = async () => {
  isInstallingF2b.value = true
  showToast("Info", "Installing Fail2Ban... This may take a minute.", "info")
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/install`, { method: 'POST' })
    if (res.ok) {
      showToast("Success", "Fail2Ban installed and started successfully", "success")
      await fetchFail2Ban()
    } else {
      showToast("Error", await res.text(), "error")
    }
  } catch (e) {
    showToast("Error", "Installation failed", "error")
  } finally {
    isInstallingF2b.value = false
  }
}

const unbanIp = async (jail, ip) => {
  showConfirm("Unban IP", `Are you sure you want to unban IP ${ip} from jail ${jail}?`, async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/unban`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ jail, ip })
      })
      if (res.ok) {
        showToast("Success", await res.json().then(data => data.message), "success")
        await fetchFail2Ban()
      } else {
        showToast("Error", await res.text(), "error")
      }
    } catch (e) {
      showToast("Error", "Failed to unban IP", "error")
    }
  })
}

// ── SPEEDTEST STATE ──
const history = ref([])
const isRunningSpeedtest = ref(false)
const isLoadingSpeedtest = ref(true)

const fetchSpeedtestHistory = async () => {
  try {
    isLoadingSpeedtest.value = true
    const res = await apiFetch(`${getActiveServerUrl()}/api/speedtest/history`)
    if (res.ok) {
      history.value = await res.json()
    }
  } catch (e) {
    showToast("Error", "Failed to fetch speedtest history", "error")
  } finally {
    isLoadingSpeedtest.value = false
  }
}

const runSpeedtest = async () => {
  isRunningSpeedtest.value = true
  showToast("Info", "Starting Speedtest... This may take a minute.", "info")
  
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/speedtest/run`, {
      method: 'POST'
    })
    
    if (res.ok) {
      showToast("Success", "Speedtest completed", "success")
      await fetchSpeedtestHistory()
    } else {
      const err = await res.text()
      showToast("Error", `Speedtest failed: ${err}`, "error")
    }
  } catch (e) {
    showToast("Error", "Failed to run speedtest", "error")
  } finally {
    isRunningSpeedtest.value = false
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
  fetchNetwork()
  fetchPorts()
  fetchSpeedtestHistory()
  fetchFail2Ban()
  pollInterval = setInterval(() => {
    fetchNetwork()
    fetchPorts()
  }, 5000)
})

onUnmounted(() => {
  clearInterval(pollInterval)
  if(scanPollInterval) clearInterval(scanPollInterval)
})
</script>

<template>
  <div class="space-y-6">
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <section class="card">
        <h2 class="card-title"><Network class="w-5 h-5 text-brand-500" /> Interfaces</h2>
        <div class="overflow-x-auto">
          <table class="w-full">
            <thead>
              <tr>
                <th class="table-th">Name / MAC</th>
                <th class="table-th">IP Address</th>
                <th class="table-th text-right">RX / TX (MB)</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="iface in networkInterfaces" :key="iface.name">
                <td class="table-td">
                  <div class="font-medium">{{ iface.name }}</div>
                  <div class="text-xs font-mono" :class="isDark ? 'text-slate-400' : 'text-slate-400'">{{ iface.mac_address }}</div>
                </td>
                <td class="table-td font-mono text-xs leading-relaxed">
                  <div v-for="ip in iface.ip_networks" :key="ip" class="mb-0.5">{{ ip }}</div>
                  <div v-if="iface.gateway" class="text-[10px] mt-1 pt-1 border-t border-slate-200 dark:border-slate-700" :class="isDark ? 'text-emerald-400/80' : 'text-emerald-600/80'">
                    Gateway: {{ iface.gateway }}
                  </div>
                </td>
                <td class="table-td text-right">
                  <div class="font-medium" :class="isDark ? 'text-green-400' : 'text-green-600'">↓ {{ (iface.rx_bytes/1048576).toFixed(2) }}</div>
                  <div class="font-medium" :class="isDark ? 'text-blue-400' : 'text-blue-600'">↑ {{ (iface.tx_bytes/1048576).toFixed(2) }}</div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section class="card">
        <h2 class="card-title"><ShieldCheck class="w-5 h-5 text-brand-500" /> Port Security</h2>
        
        <div class="mb-5 p-4 rounded-lg"
             :class="isDark ? 'bg-slate-800/50 border-slate-700' : 'bg-slate-50 border-slate-200'">
          <div class="flex items-center gap-2 mb-3">
            <Radar class="w-4 h-4" :class="isDark ? 'text-slate-400' : 'text-slate-600'" />
            <h3 class="font-semibold text-sm" :class="isDark ? 'text-slate-100' : 'text-slate-800'">Deep Scan (Nmap)</h3>
          </div>
          <div class="flex gap-2">
            <input v-model="scanTarget" type="text" placeholder="IP / localhost" class="input-field flex-1" :disabled="isScanning">
            <button @click="startScan" class="btn-primary whitespace-nowrap" :disabled="isScanning">
              <Radar class="w-4 h-4" /> {{ isScanning ? 'Scanning...' : 'Run Scan' }}
            </button>
          </div>
          <div class="mt-2 text-sm" :class="scanResult ? (isDark ? 'text-green-400' : 'text-green-600') : (isDark ? 'text-slate-400' : 'text-slate-600')">{{ scanStatusMsg }}</div>
          <pre v-if="scanResult" class="mt-3 bg-slate-900 text-slate-50 p-3 rounded text-xs overflow-x-auto">{{ scanResult }}</pre>
        </div>

        <div class="overflow-x-auto max-h-64 overflow-y-auto">
          <table class="w-full relative">
            <thead class="sticky top-0 shadow-[0_1px_0_0_#e2e8f0]" :class="isDark ? 'dark:bg-slate-900' : 'bg-white'">
              <tr>
                <th class="table-th" :class="isDark ? 'dark:bg-slate-900' : 'bg-white'">Proto</th>
                <th class="table-th" :class="isDark ? 'dark:bg-slate-900' : 'bg-white'">Local Address</th>
                <th class="table-th" :class="isDark ? 'dark:bg-slate-900' : 'bg-white'">Process</th>
                <th class="table-th" :class="isDark ? 'dark:bg-slate-900' : 'bg-white'">Action</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="port in listeningPorts" :key="port.local_address+port.protocol">
                <td class="table-td">
                  <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium" :class="port.protocol.includes('tcp') ? (isDark ? 'bg-blue-900/30 text-blue-300' : 'bg-blue-100 text-blue-800') : (isDark ? 'bg-purple-900/30 text-purple-300' : 'bg-purple-100 text-purple-800')">
                    {{ port.protocol.toUpperCase() }}
                  </span>
                </td>
                <td class="table-td font-mono text-xs">{{ port.local_address }}</td>
                <td class="table-td text-xs truncate max-w-[200px]" :title="port.process">{{ port.process }}</td>
                <td class="table-td text-right">
                  <button v-if="extractPid(port.process)" @click="killPortProcess(extractPid(port.process))" class="btn-icon-danger" title="Kill Process">
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/><path d="M8 20v2h8v-2"/><path d="m12.5 17-.5-1-.5 1h1z"/><path d="M16 20a2 2 0 0 0 1.56-3.25 8 8 0 1 0-11.12 0A2 2 0 0 0 8 20"/></svg>
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>

    <!-- ── FAIL2BAN SECTION ── -->
    <section class="card">
      <div class="flex items-center justify-between mb-4">
        <h2 class="card-title mb-0"><ShieldCheck class="w-5 h-5 text-brand-500" /> Intrusion Prevention (Fail2Ban)</h2>
        <button v-if="f2bStatus && !f2bStatus.installed" @click="installFail2Ban" class="btn-primary" :disabled="isInstallingF2b">
          <Play v-if="!isInstallingF2b" class="w-4 h-4" />
          <Activity v-else class="w-4 h-4 animate-spin" />
          {{ isInstallingF2b ? 'Installing...' : 'Install Fail2Ban' }}
        </button>
        <button v-else-if="f2bStatus && f2bStatus.installed" @click="fetchFail2Ban" class="btn-outline text-xs h-8 px-3">
          <Radar class="w-3.5 h-3.5" /> Refresh
        </button>
      </div>

      <div v-if="isLoadingF2b" class="p-6 text-center text-slate-500">Loading...</div>
      
      <div v-else-if="f2bStatus && !f2bStatus.installed" class="p-6 text-center text-slate-500 bg-slate-50 dark:bg-slate-800/50 rounded-lg">
        Fail2Ban is not installed on this server. Install it to protect services like SSH from brute-force attacks.
      </div>
      
      <div v-else-if="f2bStatus && f2bStatus.installed" class="space-y-6">
        
        <div class="flex flex-col md:flex-row gap-3 items-center">
          <button @click="openConfigModal" class="btn-primary text-xs h-8 px-3 whitespace-nowrap shrink-0 w-full md:w-auto">
            <ShieldCheck class="w-3.5 h-3.5" /> Configure Jails
          </button>
          
          <!-- Manual Ban Form -->
          <div class="grid grid-cols-[100px_1fr_auto] gap-2 w-full flex-1">
            <select v-model="formManualBan.jail" class="input-field py-1 px-2 text-xs h-8">
              <option v-for="j in f2bStatus.jails" :key="j.name" :value="j.name">{{ j.name }}</option>
            </select>
            <input v-model="formManualBan.ip" type="text" placeholder="IP Address to ban..." class="input-field py-1 px-3 text-xs h-8 min-w-0" :disabled="isBanning" @keyup.enter="banIp">
            <button @click="banIp" class="btn-danger py-1 px-3 text-xs h-8 whitespace-nowrap" :disabled="isBanning">
               {{ isBanning ? 'Banning...' : 'Ban IP' }}
            </button>
          </div>
        </div>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 items-start">
          
          <!-- Active Jails List -->
          <div class="space-y-4">
            <h3 class="text-xs font-bold uppercase tracking-wider text-slate-500">Active Jails & Blocks</h3>
            <div v-for="jail in f2bStatus.jails" :key="jail.name" class="border rounded-lg" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
              <div class="px-4 py-2 border-b bg-slate-50 dark:bg-slate-800/50 flex justify-between items-center">
                <span class="font-bold text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-700'">{{ jail.name }}</span>
                <span class="text-xs px-2 py-0.5 rounded-full" :class="isDark ? 'bg-red-900/30 text-red-400' : 'bg-red-100 text-red-600'">{{ jail.banned_ips.length }} Banned</span>
              </div>
              <div class="p-3">
                <div v-if="jail.banned_ips.length === 0" class="text-xs text-slate-500 italic">No IPs currently banned.</div>
                <div v-else class="flex flex-wrap gap-2">
                  <div v-for="ip in jail.banned_ips" :key="ip" class="flex items-center gap-2 px-2 py-1 rounded border text-xs" :class="isDark ? 'bg-slate-900 border-slate-700 text-slate-300' : 'bg-white border-slate-200 text-slate-700'">
                    <span class="font-mono">{{ ip }}</span>
                    <button @click="unbanIp(jail.name, ip)" class="text-green-600 hover:text-green-700 dark:text-green-400 dark:hover:text-green-300 font-bold uppercase" title="Unban this IP">Unban</button>
                  </div>
                </div>
              </div>
            </div>
            <div v-if="f2bStatus.jails.length === 0" class="text-sm text-slate-500 italic p-4 text-center border border-dashed rounded-lg">No active jails found. Click 'Configure Jails' to enable them.</div>
          </div>

          <!-- Live Activity Log -->
          <div class="space-y-2">
            <div class="flex justify-between items-center">
              <h3 class="text-xs font-bold uppercase tracking-wider text-slate-500">Fail2Ban Logs</h3>
              <button @click="fetchF2bLogs" class="text-xs text-brand-500 hover:underline">Refresh</button>
            </div>
            <div class="bg-black/90 text-slate-300 font-mono text-[10px] p-4 rounded-lg overflow-y-auto h-[400px] shadow-inner leading-relaxed">
              <div v-if="f2bLogs.length === 0" class="text-slate-500 italic">No recent logs found...</div>
              <div v-for="(line, idx) in f2bLogs" :key="idx" 
                   class="py-0.5 hover:bg-white/5 px-1"
                   :class="{'text-red-400': line.includes('Ban'), 'text-green-400': line.includes('Unban'), 'text-blue-400': line.includes('Found')}">
                {{ line }}
              </div>
            </div>
          </div>

        </div>
      </div>
    </section>

    <!-- Modal: Jail Config -->
    <Teleport to="body">
      <div v-if="showConfigModal" class="fixed inset-0 z-[100] backdrop-blur-sm flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
        <div class="rounded-xl shadow-2xl w-full max-w-4xl overflow-hidden flex flex-col max-h-[90vh]" :class="isDark ? 'bg-slate-800' : 'bg-white'">
          <div class="p-4 border-b flex justify-between items-center shrink-0" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'">
            <h3 class="font-bold flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'">
              <ShieldCheck class="w-4 h-4 text-brand-500"/> Jail Configuration (/etc/fail2ban/jail.local)
            </h3>
            <button @click="showConfigModal = false" class="text-slate-400 hover:text-slate-200">✕</button>
          </div>
          
          <div class="flex-1 overflow-hidden flex flex-col md:flex-row">
            <!-- Sidebar: Jail List -->
            <div class="w-full md:w-1/3 border-r overflow-y-auto p-4 space-y-2" :class="isDark ? 'border-slate-700 bg-slate-900/30' : 'border-slate-200 bg-slate-50'">
              <button @click="createNewJail" class="w-full btn-outline border-dashed text-xs py-2 mb-4">+ Create Custom Jail</button>
              <div v-for="j in f2bConfig" :key="j.name" 
                   @click="editJail(j)"
                   class="p-3 border rounded-lg cursor-pointer transition-colors flex justify-between items-center"
                   :class="[
                     formConfig.name === j.name ? (isDark ? 'bg-brand-900/30 border-brand-500' : 'bg-brand-50 border-brand-400') : (isDark ? 'border-slate-700 hover:bg-slate-700' : 'border-slate-200 hover:bg-white'),
                     !j.enabled ? 'opacity-60' : ''
                   ]">
                <div>
                  <div class="font-bold text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-700'">{{ j.name }}</div>
                  <div class="text-[10px] text-slate-500">{{ j.port || 'Any port' }}</div>
                </div>
                <div class="w-2 h-2 rounded-full" :class="j.enabled ? 'bg-green-500' : 'bg-slate-300 dark:bg-slate-600'"></div>
              </div>
            </div>

            <!-- Editor Pane -->
            <div class="flex-1 p-6 overflow-y-auto">
              <div v-if="!formConfig.name" class="h-full flex items-center justify-center text-slate-500 text-sm">
                Select a jail from the list or create a new one to edit.
              </div>
              <div v-else class="space-y-4">
                <div class="flex justify-between items-center">
                  <h3 class="text-lg font-bold">{{ formConfig.name === 'DEFAULT' ? 'Global Settings' : `Edit Jail: ${formConfig.name}` }}</h3>
                  <label class="flex items-center gap-2 cursor-pointer" v-if="formConfig.name !== 'DEFAULT'">
                    <span class="text-sm font-semibold">Enable</span>
                    <input type="checkbox" v-model="formConfig.enabled" class="rounded text-brand-600 focus:ring-brand-500" />
                  </label>
                </div>

                <div class="grid grid-cols-2 gap-4">
                  <div v-if="formConfig.name !== 'DEFAULT'">
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Jail Name</label>
                    <input v-model="formConfig.name" type="text" class="input-field w-full text-sm" :disabled="f2bConfig.find(x => x.name === formConfig.name) && formConfig.name !== 'custom-'" />
                  </div>
                  <div v-if="formConfig.name !== 'DEFAULT'">
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Port (e.g. ssh, http, 8080)</label>
                    <input v-model="formConfig.port" type="text" class="input-field w-full text-sm" />
                  </div>
                  <div>
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Max Retry</label>
                    <input v-model="formConfig.maxretry" type="text" placeholder="e.g. 5" class="input-field w-full text-sm" />
                  </div>
                  <div>
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Find Time</label>
                    <input v-model="formConfig.findtime" type="text" placeholder="e.g. 10m" class="input-field w-full text-sm" />
                  </div>
                  <div>
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Ban Time</label>
                    <input v-model="formConfig.bantime" type="text" placeholder="e.g. 1h" class="input-field w-full text-sm" />
                  </div>
                  <div class="col-span-2" v-if="formConfig.name !== 'DEFAULT'">
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Log Path</label>
                    <input v-model="formConfig.logpath" type="text" placeholder="e.g. /var/log/auth.log" class="input-field w-full text-sm font-mono" />
                  </div>
                  <div class="col-span-2" v-if="formConfig.name !== 'DEFAULT'">
                    <label class="block text-xs font-semibold mb-1 text-slate-500">Filter (from /etc/fail2ban/filter.d/)</label>
                    <input v-model="formConfig.filter" type="text" placeholder="e.g. sshd or nginx-http-auth" class="input-field w-full text-sm font-mono" />
                  </div>
                </div>

                <div class="pt-4 flex justify-end">
                  <button @click="saveJailConfig" class="btn-primary">Save & Reload Service</button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- ── SPEEDTEST SECTION ── -->
    <section class="card">
      <div class="flex items-center justify-between mb-6">
        <h2 class="card-title mb-0"><Activity class="w-5 h-5 text-brand-500" /> Network Speedtest</h2>
        <button @click="runSpeedtest" class="btn-primary" :disabled="isRunningSpeedtest">
          <Play v-if="!isRunningSpeedtest" class="w-4 h-4" />
          <Activity v-else class="w-4 h-4 animate-pulse" />
          {{ isRunningSpeedtest ? 'Testing...' : 'Run Speedtest' }}
        </button>
      </div>

      <div class="overflow-x-auto">
        <table class="w-full relative">
          <thead class="border-b-2" :class="isDark ? 'bg-slate-800/50 border-slate-700' : 'bg-slate-50 border-slate-200'">
            <tr>
              <th class="table-th">Date &amp; Time</th>
              <th class="table-th">Download</th>
              <th class="table-th">Upload</th>
              <th class="table-th">Ping</th>
              <th class="table-th">Server</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="isLoadingSpeedtest">
              <td colspan="5" class="text-center p-8 text-slate-500">Loading history...</td>
            </tr>
            <tr v-else-if="history.length === 0">
              <td colspan="5" class="text-center p-8 text-slate-500">No speedtest history found. Run a test to begin.</td>
            </tr>
            <tr v-else v-for="item in history" :key="item.id" class="border-b transition-colors" :class="isDark ? 'hover:bg-slate-800/50 border-slate-700' : 'hover:bg-slate-50 border-slate-100'">
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