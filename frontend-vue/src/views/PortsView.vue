<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { Network, Radar, ShieldCheck, Activity, Play, History, Download, Upload, Server } from 'lucide-vue-next'
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
const isLoadingF2b = ref(true)
const isInstallingF2b = ref(false)

const fetchFail2Ban = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/status`)
    if (res.ok) {
      f2bStatus.value = await res.json()
    }
  } catch (e) {
  } finally {
    isLoadingF2b.value = false
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
                  <div v-for="ip in iface.ip_networks" :key="ip">{{ ip }}</div>
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
      
      <div v-else-if="f2bStatus && f2bStatus.installed" class="space-y-4">
        <div v-for="jail in f2bStatus.jails" :key="jail.name" class="border rounded-lg" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
          <div class="px-4 py-2 border-b bg-slate-50 dark:bg-slate-800/50 flex justify-between items-center">
            <span class="font-bold text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-700'">Jail: {{ jail.name }}</span>
            <span class="text-xs px-2 py-0.5 rounded-full" :class="isDark ? 'bg-red-900/30 text-red-400' : 'bg-red-100 text-red-600'">{{ jail.banned_ips.length }} Banned</span>
          </div>
          <div class="p-3">
            <div v-if="jail.banned_ips.length === 0" class="text-sm text-slate-500 italic">No IPs currently banned.</div>
            <div v-else class="flex flex-wrap gap-2">
              <div v-for="ip in jail.banned_ips" :key="ip" class="flex items-center gap-2 px-2 py-1 rounded border text-sm" :class="isDark ? 'bg-slate-900 border-slate-700 text-slate-300' : 'bg-white border-slate-200 text-slate-700'">
                <span class="font-mono">{{ ip }}</span>
                <button @click="unbanIp(jail.name, ip)" class="text-green-600 hover:text-green-700 dark:text-green-400 dark:hover:text-green-300 font-bold text-xs uppercase" title="Unban this IP">
                  Unban
                </button>
              </div>
            </div>
          </div>
        </div>
        <div v-if="f2bStatus.jails.length === 0" class="text-sm text-slate-500 italic p-4 text-center">Fail2Ban is active but no jails are reporting status. Check /etc/fail2ban/jail.local</div>
      </div>
    </section>

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