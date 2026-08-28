<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { Network, Radar, ShieldCheck } from 'lucide-vue-next'

import { useToastStore } from '../stores/toastStore'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast, showConfirm } = useToastStore()

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

onMounted(() => {
  fetchNetwork()
  fetchPorts()
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
                <div class="text-xs font-mono text-slate-400">{{ iface.mac_address }}</div>
              </td>
              <td class="table-td font-mono text-xs leading-relaxed">
                <div v-for="ip in iface.ip_networks" :key="ip">{{ ip }}</div>
              </td>
              <td class="table-td text-right">
                <div class="text-green-600 font-medium">↓ {{ (iface.rx_bytes/1048576).toFixed(2) }}</div>
                <div class="text-blue-600 font-medium">↑ {{ (iface.tx_bytes/1048576).toFixed(2) }}</div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <section class="card">
      <h2 class="card-title"><ShieldCheck class="w-5 h-5 text-brand-500" /> Port Security</h2>
      
      <div class="mb-5 p-4 bg-slate-50 border border-slate-200 rounded-lg">
        <div class="flex items-center gap-2 mb-3">
          <Radar class="w-4 h-4 text-slate-600" />
          <h3 class="font-semibold text-sm">Deep Scan (Nmap)</h3>
        </div>
        <div class="flex gap-2">
          <input v-model="scanTarget" type="text" placeholder="IP / localhost" class="input-field flex-1" :disabled="isScanning">
          <button @click="startScan" class="btn-primary whitespace-nowrap" :disabled="isScanning">
            <Radar class="w-4 h-4" /> {{ isScanning ? 'Scanning...' : 'Run Scan' }}
          </button>
        </div>
        <div class="mt-2 text-sm" :class="scanResult ? 'text-green-600' : 'text-slate-600'">{{ scanStatusMsg }}</div>
        <pre v-if="scanResult" class="mt-3 bg-slate-900 text-slate-50 p-3 rounded text-xs overflow-x-auto">{{ scanResult }}</pre>
      </div>

      <div class="overflow-x-auto max-h-64 overflow-y-auto">
        <table class="w-full relative">
          <thead class="sticky top-0 bg-white shadow-[0_1px_0_0_#e2e8f0]">
            <tr>
              <th class="table-th bg-white">Proto</th>
              <th class="table-th bg-white">Local Address</th>
              <th class="table-th bg-white">Process</th>
              <th class="table-th bg-white text-right">Action</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="port in listeningPorts" :key="port.local_address+port.protocol">
              <td class="table-td">
                <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium" :class="port.protocol.includes('tcp') ? 'bg-blue-100 text-blue-800' : 'bg-purple-100 text-purple-800'">
                  {{ port.protocol.toUpperCase() }}
                </span>
              </td>
              <td class="table-td font-mono text-xs">{{ port.local_address }}</td>
              <td class="table-td text-xs truncate max-w-[200px]" :title="port.process">{{ port.process }}</td>
              <td class="table-td text-right">
                <button v-if="extractPid(port.process)" @click="killPortProcess(extractPid(port.process))" class="btn-icon-red" title="Kill Process">
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/><path d="M8 20v2h8v-2"/><path d="m12.5 17-.5-1-.5 1h1z"/><path d="M16 20a2 2 0 0 0 1.56-3.25 8 8 0 1 0-11.12 0A2 2 0 0 0 8 20"/></svg>
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

  </div>
</template>