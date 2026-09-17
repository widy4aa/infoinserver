<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import {
  Box, Play, Square, RefreshCw, Trash2, FileText, Plus,
  Layers, Settings2, Terminal, ChevronDown, ChevronRight,
  AlertCircle, CheckCircle2, Loader2, ExternalLink, Upload,
  RotateCcw, Scaling, Eye, Pencil, X, Save, ZoomIn,
  Server, HardDrive, Link, Cpu
} from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl, addServer, servers, getToken, activeServerId } = useServerStore()
const { showConfirm, showToast } = useToastStore()
const { isDark } = useThemeStore()

// ── Active Tab ─────────────────────────────────────────────────────
const activeTab = ref('containers') // 'containers' | 'compose' | 'deploy' | 'vms'

// ── Runtime ────────────────────────────────────────────────────────
const runtime = ref(null)
const isLoadingRuntime = ref(true)

// ── Containers ─────────────────────────────────────────────────────
const containers = ref([])
const isLoadingContainers = ref(false)
const showOnlyRunning = ref(false)
const containerSearch = ref('')

// ── Modals ─────────────────────────────────────────────────────────
const logsModal = ref({ open: false, title: '', logs: '', loading: false })
const inspectModal = ref({ open: false, title: '', data: null })
const yamlModal = ref({ open: false, projectName: '', yaml: '', editing: false, saving: false })

// ── Compose ────────────────────────────────────────────────────────
const composeProjects = ref([])
const isLoadingCompose = ref(false)
const expandedProjects = ref(new Set())
const scaleModal = ref({ open: false, projectName: '', service: '', count: 1 })

// ── Deploy Forms ────────────────────────────────────────────────────
const deployTab = ref('container') // 'container' | 'compose'

// Container deploy form
const dcName = ref('')
const dcImage = ref('')
const dcPorts = ref('')
const dcEnv = ref('')
const dcVolumes = ref('')
const dcRestart = ref('')
const isDeployingContainer = ref(false)

// Compose deploy form
const cpName = ref('')
const cpYaml = ref(`services:
  app:
    image: nginx:alpine
    ports:
      - "8080:80"
    restart: unless-stopped
`)
const isDeployingCompose = ref(false)

// ── VM (systemd-nspawn) State ────────────────────────────────────────────────
const vms = ref([])
const isLoadingVms = ref(false)
const vmForm = ref({
  name: '',
  distro: 'ubuntu-2404',
  username: '',
  password: '',
  backend_port: 8081,
  host_ip: '',
})
const vmDeployModal = ref({
  open: false,
  logs: [],
  status: 'idle',  // 'idle' | 'running' | 'done' | 'error'
  result: null,
})
let vmWs = null

let pollInterval = null

// ── Computed ───────────────────────────────────────────────────────
const filteredContainers = computed(() => {
  let list = containers.value
  if (showOnlyRunning.value) list = list.filter(c => c.state === 'running')
  if (containerSearch.value) {
    const q = containerSearch.value.toLowerCase()
    list = list.filter(c =>
      c.name.toLowerCase().includes(q) ||
      c.image.toLowerCase().includes(q) ||
      (c.compose_project || '').toLowerCase().includes(q)
    )
  }
  return list
})

const standaloneContainers = computed(() =>
  filteredContainers.value.filter(c => !c.compose_project)
)

const composeContainers = computed(() =>
  filteredContainers.value.filter(c => c.compose_project)
)

// ── Runtime ────────────────────────────────────────────────────────
const fetchRuntime = async () => {
  isLoadingRuntime.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/container/runtime`)
    if (res.ok) runtime.value = await res.json()
  } catch (e) {}
  finally { isLoadingRuntime.value = false }
}

const refreshRuntime = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/container/runtime/refresh`, { method: 'POST' })
    const data = await res.json()
    if (res.ok) {
      showToast('Success', data.message, 'success')
      await fetchRuntime()
    } else {
      showToast('Error', data, 'error')
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  }
}

// ── Containers ─────────────────────────────────────────────────────
const fetchContainers = async () => {
  if (!runtime.value?.available) return
  isLoadingContainers.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/container/list`)
    if (res.ok) containers.value = await res.json()
  } catch (e) {}
  finally { isLoadingContainers.value = false }
}

const doAction = async (action, id, name) => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/container/${action}/${id}`, { method: 'POST' })
    const data = await res.json()
    if (res.ok) {
      showToast('Success', data.message, 'success')
      await fetchContainers()
    } else {
      showToast('Error', typeof data === 'string' ? data : data.message || JSON.stringify(data), 'error')
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  }
}

const performAction = (action, id, name) => {
  if (action === 'rm') {
    showConfirm('Delete Container', `Delete container "${name}"?`, () => doAction(action, id, name))
  } else {
    doAction(action, id, name)
  }
}

const viewLogs = async (id, name) => {
  logsModal.value = { open: true, title: `Logs — ${name}`, logs: '', loading: true }
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/container/logs/${id}?tail=200`)
    const data = await res.json()
    logsModal.value.logs = data.logs || 'No logs.'
  } catch (e) {
    logsModal.value.logs = `Error: ${e.message}`
  } finally {
    logsModal.value.loading = false
  }
}

const viewInspect = async (id, name) => {
  inspectModal.value = { open: true, title: `Inspect — ${name}`, data: null }
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/container/inspect/${id}`)
    inspectModal.value.data = await res.json()
  } catch (e) {
    inspectModal.value.data = { error: e.message }
  }
}

// ── Deploy Container ───────────────────────────────────────────────
const deployContainer = async () => {
  if (!dcName.value || !dcImage.value) {
    showToast('Warning', 'Name and Image are required', 'warning')
    return
  }
  isDeployingContainer.value = true
  try {
    const ports = dcPorts.value ? dcPorts.value.split(',').map(s => s.trim()).filter(Boolean) : []
    const env_vars = dcEnv.value ? dcEnv.value.split('\n').map(s => s.trim()).filter(Boolean) : []
    const volumes = dcVolumes.value ? dcVolumes.value.split('\n').map(s => s.trim()).filter(Boolean) : []
    const res = await apiFetch(`${getActiveServerUrl()}/api/container/create`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name: dcName.value,
        image: dcImage.value,
        ports, env_vars, volumes,
        restart_policy: dcRestart.value || null
      })
    })
    const data = await res.json()
    if (res.ok) {
      showToast('Success', data.message, 'success')
      dcName.value = ''; dcImage.value = ''; dcPorts.value = ''
      dcEnv.value = ''; dcVolumes.value = ''; dcRestart.value = ''
      await fetchContainers()
      activeTab.value = 'containers'
    } else {
      showToast('Error', typeof data === 'string' ? data : data.message || JSON.stringify(data), 'error')
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    isDeployingContainer.value = false
  }
}

// ── Compose Projects ───────────────────────────────────────────────
const fetchCompose = async () => {
  if (!runtime.value?.available) return
  isLoadingCompose.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/compose/projects`)
    if (res.ok) composeProjects.value = await res.json()
  } catch (e) {}
  finally { isLoadingCompose.value = false }
}

const deployCompose = async () => {
  if (!cpName.value || !cpYaml.value.trim()) {
    showToast('Warning', 'Project name and YAML are required', 'warning')
    return
  }
  isDeployingCompose.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/compose/deploy`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: cpName.value, yaml: cpYaml.value })
    })
    const data = await res.json()
    if (res.ok) {
      showToast('Success', data.message, 'success')
      cpName.value = ''
      await fetchCompose()
      await fetchContainers()
      activeTab.value = 'compose'
    } else {
      showToast('Error', typeof data === 'string' ? data : data.message || JSON.stringify(data), 'error')
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    isDeployingCompose.value = false
  }
}

const composeAction = async (action, name) => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/compose/${name}/${action}`, { method: 'POST' })
    const data = await res.json()
    if (res.ok) {
      showToast('Success', data.message, 'success')
      await fetchCompose()
      await fetchContainers()
    } else {
      showToast('Error', typeof data === 'string' ? data : data.message || JSON.stringify(data), 'error')
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  }
}

const deleteCompose = (name, withVolumes = false) => {
  showConfirm(
    'Delete Compose Project',
    `Delete project "${name}"? This will stop all services.`,
    async () => {
      try {
        const res = await apiFetch(`${getActiveServerUrl()}/api/compose/${name}?remove_volumes=${withVolumes}`, { method: 'DELETE' })
        const data = await res.json()
        if (res.ok) {
          showToast('Success', data.message, 'success')
          await fetchCompose()
          await fetchContainers()
        } else {
          showToast('Error', typeof data === 'string' ? data : data.message, 'error')
        }
      } catch (e) {
        showToast('Error', e.message, 'error')
      }
    }
  )
}

const viewComposeLogs = async (name, service = null) => {
  const title = service ? `Logs — ${name} / ${service}` : `Logs — ${name} (all services)`
  logsModal.value = { open: true, title, logs: '', loading: true }
  try {
    const url = service
      ? `${getActiveServerUrl()}/api/compose/${name}/logs?service=${service}&tail=200`
      : `${getActiveServerUrl()}/api/compose/${name}/logs?tail=200`
    const res = await apiFetch(url)
    const data = await res.json()
    logsModal.value.logs = data.logs || 'No logs.'
  } catch (e) {
    logsModal.value.logs = `Error: ${e.message}`
  } finally {
    logsModal.value.loading = false
  }
}

const openYamlModal = async (name) => {
  yamlModal.value = { open: true, projectName: name, yaml: '', editing: false, saving: false }
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/compose/${name}/yaml`)
    const data = await res.json()
    yamlModal.value.yaml = data.yaml || ''
  } catch (e) {
    yamlModal.value.yaml = `Error: ${e.message}`
  }
}

const saveYaml = async () => {
  yamlModal.value.saving = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/compose/${yamlModal.value.projectName}/yaml`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ yaml: yamlModal.value.yaml })
    })
    const data = await res.json()
    if (res.ok) {
      showToast('Success', data.message, 'success')
      yamlModal.value.editing = false
      await fetchCompose()
      await fetchContainers()
    } else {
      showToast('Error', typeof data === 'string' ? data : data.message, 'error')
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  } finally {
    yamlModal.value.saving = false
  }
}

const openScaleModal = (projectName, service, currentCount) => {
  scaleModal.value = { open: true, projectName, service, count: currentCount || 1 }
}

const doScale = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/compose/${scaleModal.value.projectName}/scale`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ service: scaleModal.value.service, count: scaleModal.value.count })
    })
    const data = await res.json()
    if (res.ok) {
      showToast('Success', data.message, 'success')
      scaleModal.value.open = false
      await fetchCompose()
      await fetchContainers()
    } else {
      showToast('Error', typeof data === 'string' ? data : data.message, 'error')
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  }
}

const toggleProject = (name) => {
  if (expandedProjects.value.has(name)) {
    expandedProjects.value.delete(name)
  } else {
    expandedProjects.value.add(name)
  }
  expandedProjects.value = new Set(expandedProjects.value)
}

// ── VM functions ───────────────────────────────────────────────────

const distroLabel = (distro) => {
  const map = { 'ubuntu-2404': 'Ubuntu 24.04 LTS', 'arch': 'Arch Linux' }
  return map[distro] || distro
}

const distroColor = (distro) => {
  if (distro === 'ubuntu-2404') return isDark.value ? 'bg-orange-900/40 text-orange-300 border-orange-700' : 'bg-orange-100 text-orange-700 border-orange-200'
  if (distro === 'arch')        return isDark.value ? 'bg-blue-900/40 text-blue-300 border-blue-700'       : 'bg-blue-100 text-blue-700 border-blue-200'
  return isDark.value ? 'bg-slate-700 text-slate-300' : 'bg-slate-100 text-slate-600'
}

const vmStateColor = (state) => {
  if (state === 'running') return isDark.value ? 'text-green-300 bg-green-900/30' : 'text-green-700 bg-green-100'
  return isDark.value ? 'text-slate-400 bg-slate-700' : 'text-slate-600 bg-slate-100'
}

const fetchVms = async () => {
  isLoadingVms.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/vm/list`)
    if (res.ok) {
      const data = await res.json()
      vms.value = data.vms || []
    }
  } catch (e) {}
  finally { isLoadingVms.value = false }
}

const vmAction = async (action, name) => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/vm/${name}/${action}`, { method: 'POST' })
    const data = await res.json()
    if (res.ok) {
      showToast('Success', data.message, 'success')
      await fetchVms()
    } else {
      showToast('Error', data.message || data, 'error')
    }
  } catch (e) {
    showToast('Error', e.message, 'error')
  }
}

const deleteVm = (name) => {
  showConfirm(
    'Delete VM',
    `Delete VM "${name}"? This will remove the entire OS container and its data.`,
    async () => {
      try {
        const res = await apiFetch(`${getActiveServerUrl()}/api/vm/${name}`, { method: 'DELETE' })
        const data = await res.json()
        if (res.ok) {
          showToast('Success', data.message, 'success')
          await fetchVms()
        } else {
          showToast('Error', data.message || data, 'error')
        }
      } catch (e) {
        showToast('Error', e.message, 'error')
      }
    }
  )
}

const addVmToDashboard = (vm) => {
  const url = `http://${vm.host_ip}:${vm.backend_port}`
  const alreadyExists = servers.value.some(s => s.url === url)
  if (alreadyExists) {
    showToast('Info', `Server ${url} already in dashboard.`, 'info')
    return
  }
  const id = 'vm-' + vm.id.slice(0, 8)
  addServer(`VM: ${vm.name} (${distroLabel(vm.distro)})`, url, id)
  showToast('Success', `VM '${vm.name}' added to dashboard!`, 'success')
}

// Suggest next free port
const suggestNextPort = () => {
  const usedPorts = vms.value.map(v => v.backend_port)
  let port = 8081
  while (usedPorts.includes(port)) port++
  vmForm.value.backend_port = port
}

// Get host IP from server URL
const inferHostIp = () => {
  try {
    const url = new URL(getActiveServerUrl())
    vmForm.value.host_ip = url.hostname
  } catch { vmForm.value.host_ip = '127.0.0.1' }
}

const openVmDeploy = () => {
  suggestNextPort()
  inferHostIp()
  vmDeployModal.value = { open: true, logs: [], status: 'idle', result: null }
}

const closeVmDeploy = () => {
  if (vmWs) { vmWs.close(); vmWs = null }
  vmDeployModal.value.open = false
}

const deployVm = () => {
  const { name, distro, username, password, backend_port, host_ip } = vmForm.value
  if (!name || !username || !password || !backend_port || !host_ip) {
    showToast('Warning', 'All fields are required.', 'warning')
    return
  }
  if (!/^[a-zA-Z0-9_-]+$/.test(name)) {
    showToast('Warning', 'VM name: only letters, numbers, dashes, underscores.', 'warning')
    return
  }

  vmDeployModal.value.logs = []
  vmDeployModal.value.status = 'running'
  vmDeployModal.value.result = null

  // Build WebSocket URL — go through the Bun proxy which handles /api/* prefix
  const serverUrl = getActiveServerUrl()
  const wsBase = serverUrl.replace(/^http/, 'ws')
  const params = new URLSearchParams({
    name, distro, username, password,
    backend_port: String(backend_port),
    host_ip,
  })
  const wsUrl = `${wsBase}/api/vm/create/ws?${params}&token=${getToken(activeServerId.value) || ''}`

  if (vmWs) vmWs.close()
  vmWs = new WebSocket(wsUrl)

  vmWs.onmessage = (evt) => {
    try {
      const msg = JSON.parse(evt.data)
      if (msg.event === 'done') {
        vmDeployModal.value.status = 'done'
        vmDeployModal.value.logs.push({ event: 'done', msg: msg.msg })
        fetchVms()
      } else if (msg.event === 'error') {
        vmDeployModal.value.status = 'error'
        vmDeployModal.value.logs.push({ event: 'error', msg: msg.msg })
      } else if (msg.event === 'info') {
        try { vmDeployModal.value.result = JSON.parse(msg.msg) } catch {}
      } else {
        vmDeployModal.value.logs.push({ event: msg.event, msg: msg.msg })
      }
    } catch {
      vmDeployModal.value.logs.push({ event: 'log', msg: evt.data })
    }
    // Auto-scroll terminal
    setTimeout(() => {
      const el = document.getElementById('vm-deploy-log')
      if (el) el.scrollTop = el.scrollHeight
    }, 30)
  }

  vmWs.onerror = () => {
    vmDeployModal.value.status = 'error'
    vmDeployModal.value.logs.push({ event: 'error', msg: 'WebSocket connection failed.' })
  }

  vmWs.onclose = () => {
    if (vmDeployModal.value.status === 'running') {
      vmDeployModal.value.status = 'error'
      vmDeployModal.value.logs.push({ event: 'error', msg: 'Connection closed unexpectedly.' })
    }
  }
}

// Get active token for WS auth — diambil dari serverStore yang sudah diimport di atas

const stateColor = (state) => {
  if (state === 'running') return isDark.value ? 'text-green-300 bg-green-900/30' : 'text-green-700 bg-green-100'
  if (state === 'exited' || state === 'stopped') return isDark.value ? 'text-slate-400 bg-slate-800' : 'text-slate-600 bg-slate-100'
  if (state === 'paused') return isDark.value ? 'text-amber-300 bg-amber-900/30' : 'text-amber-700 bg-amber-100'
  return isDark.value ? 'text-slate-400 bg-slate-800' : 'text-slate-500 bg-slate-100'
}

const projectStatusColor = (status) => {
  if (status === 'running') return isDark.value ? 'text-green-300 bg-green-900/30 border-green-800' : 'text-green-700 bg-green-100 border-green-200'
  if (status === 'partial') return isDark.value ? 'text-amber-300 bg-amber-900/30 border-amber-800' : 'text-amber-700 bg-amber-100 border-amber-200'
  return isDark.value ? 'text-slate-400 bg-slate-800 border-slate-700' : 'text-slate-600 bg-slate-100 border-slate-200'
}

onMounted(async () => {
  await fetchRuntime()
  await Promise.all([fetchContainers(), fetchCompose(), fetchVms()])
  pollInterval = setInterval(async () => {
    await fetchContainers()
    if (activeTab.value === 'compose') await fetchCompose()
    if (activeTab.value === 'vms') await fetchVms()
  }, 6000)
})

onUnmounted(() => {
  clearInterval(pollInterval)
  if (vmWs) { vmWs.close(); vmWs = null }
})
</script>

<template>
  <div class="space-y-4">

    <!-- ── Runtime Badge ─────────────────────────────────────────── -->
    <div class="flex items-center justify-between flex-wrap gap-3">
      <div class="flex items-center gap-3">
        <div v-if="isLoadingRuntime" class="flex items-center gap-1.5 text-slate-400 text-sm">
          <Loader2 class="w-4 h-4 animate-spin" /> Detecting runtime...
        </div>
        <template v-else-if="runtime?.available">
          <span class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-sm font-semibold bg-brand-100 dark:bg-slate-700 text-brand-800 dark:text-brand-300 border border-brand-200 dark:border-slate-600">
            <Box class="w-3.5 h-3.5" />
            {{ runtime.kind }} {{ runtime.version }}
          </span>
          <span v-if="runtime.compose_available" class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 dark:bg-green-900/60 text-green-700 dark:text-green-300">
            <Layers class="w-3 h-3" /> Compose available
          </span>
          <span v-else class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-amber-100 dark:bg-amber-900/60 text-amber-700 dark:text-amber-300">
            <AlertCircle class="w-3 h-3" /> No compose
          </span>
        </template>
        <div v-else class="flex items-center gap-2 text-red-500 text-sm">
          <AlertCircle class="w-4 h-4" />
          No container runtime detected
          <button @click="refreshRuntime" class="btn-secondary text-xs">Retry</button>
        </div>
      </div>

      <div class="flex gap-2">
        <button @click="refreshRuntime" class="btn-secondary text-xs">
          <RefreshCw class="w-3.5 h-3.5" /> Refresh Runtime
        </button>
        <button @click="async () => { await fetchContainers(); await fetchCompose() }" class="btn-secondary text-xs">
          <RefreshCw class="w-3.5 h-3.5" /> Sync
        </button>
      </div>
    </div>

    <!-- ── Main Tabs ─────────────────────────────────────────────── -->
    <div class="bg-white dark:bg-slate-800 rounded-xl border border-slate-200 dark:border-slate-700 shadow-sm overflow-hidden">
      <div class="flex border-b border-slate-200 dark:border-slate-700 bg-slate-50/50 dark:bg-slate-800/50 px-2 pt-2 gap-1">
        <button v-for="tab in [
          { id: 'containers', label: 'Containers', icon: Box },
          { id: 'compose', label: 'Compose', icon: Layers },
          { id: 'deploy', label: 'Deploy', icon: Plus },
          { id: 'vms', label: 'VMs', icon: Server },
        ]" :key="tab.id"
          @click="activeTab = tab.id"
          class="flex items-center gap-1.5 px-3 py-2 text-sm font-medium rounded-t-lg border-b-2 transition-colors"
          :class="activeTab === tab.id
            ? 'border-brand-500 text-brand-700 bg-white dark:bg-slate-800 dark:border-brand-400 dark:text-brand-300'
            : 'border-transparent text-slate-500 hover:text-slate-700 hover:bg-slate-100 dark:text-slate-400 dark:hover:text-slate-200 dark:hover:bg-slate-800'"
        >
          <component :is="tab.icon" class="w-4 h-4" />
          {{ tab.label }}
          <span v-if="tab.id === 'containers' && containers.length" class="ml-1 px-1.5 py-0.5 rounded-full text-[10px] font-bold bg-slate-200 dark:bg-slate-700 text-slate-600 dark:text-slate-300">{{ containers.length }}</span>
          <span v-if="tab.id === 'compose' && composeProjects.length" class="ml-1 px-1.5 py-0.5 rounded-full text-[10px] font-bold bg-slate-200 dark:bg-slate-700 text-slate-600 dark:text-slate-300">{{ composeProjects.length }}</span>
          <span v-if="tab.id === 'vms' && vms.length" class="ml-1 px-1.5 py-0.5 rounded-full text-[10px] font-bold bg-slate-200 dark:bg-slate-700 text-slate-600 dark:text-slate-300">{{ vms.length }}</span>
        </button>
      </div>

      <div class="p-4">

        <!-- ── Tab: Containers ─────────────────────────────────── -->
        <div v-if="activeTab === 'containers'" class="space-y-3">
          <!-- Toolbar -->
          <div class="flex flex-wrap gap-2 items-center">
            <input v-model="containerSearch" type="text" placeholder="Search name / image / project..." class="input-field max-w-xs text-sm" />
            <label class="flex items-center gap-1.5 text-sm text-slate-600 dark:text-slate-300 cursor-pointer select-none">
              <input v-model="showOnlyRunning" type="checkbox" class="rounded" />
              Running only
            </label>
          </div>

          <!-- No runtime -->
          <div v-if="!runtime?.available" class="text-center py-8 text-slate-400 text-sm">
            <AlertCircle class="w-8 h-8 mx-auto mb-2 opacity-40" />
            No container runtime available
          </div>

          <div v-else-if="isLoadingContainers && containers.length === 0" class="flex justify-center py-8">
            <Loader2 class="w-6 h-6 animate-spin text-brand-500" />
          </div>

          <div v-else class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead class="bg-slate-50 dark:bg-slate-800/50 border-b-2 border-slate-200 dark:border-slate-700">
                <tr>
                  <th class="table-th">Name</th>
                  <th class="table-th">Image</th>
                  <th class="table-th">Status</th>
                  <th class="table-th">Ports</th>
                  <th class="table-th">Project</th>
                  <th class="table-th text-right">Actions</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-100 dark:divide-slate-700">
                <tr v-for="c in filteredContainers" :key="c.id" class="hover:bg-slate-50 dark:hover:bg-slate-700/50 group">
                  <td class="table-td font-medium text-slate-800 dark:text-slate-100">{{ c.name }}</td>
                  <td class="table-td text-slate-500 dark:text-slate-400 text-xs font-mono max-w-[180px] truncate" :title="c.image">{{ c.image }}</td>
                  <td class="table-td">
                    <div class="flex items-center gap-1.5">
                      <span class="relative flex h-2 w-2">
                        <span v-if="c.state === 'running'" class="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                        <span class="relative inline-flex rounded-full h-2 w-2" :class="c.state === 'running' ? 'bg-green-500' : 'bg-slate-400'"></span>
                      </span>
                      <span class="px-1.5 py-0.5 rounded text-[10px] font-semibold uppercase" :class="stateColor(c.state)">{{ c.state }}</span>
                    </div>
                    <div class="text-[10px] text-slate-400 mt-0.5">{{ c.status }}</div>
                  </td>
                  <td class="table-td">
                    <div v-if="c.ports.length" class="flex flex-wrap gap-0.5">
                      <span v-for="p in c.ports" :key="p" class="text-[10px] font-mono bg-slate-100 dark:bg-slate-700 px-1 rounded">{{ p }}</span>
                    </div>
                    <span v-else class="text-xs text-slate-300 dark:text-slate-600">—</span>
                  </td>
                  <td class="table-td">
                    <span v-if="c.compose_project" class="inline-flex items-center gap-1 text-[10px] font-medium bg-brand-50 text-brand-700 border border-brand-100 dark:bg-brand-900/30 dark:text-brand-300 dark:border-brand-800 px-1.5 py-0.5 rounded-full">
                      <Layers class="w-2.5 h-2.5" />{{ c.compose_project }}
                    </span>
                    <span v-else class="text-xs text-slate-300 dark:text-slate-600">—</span>
                  </td>
                  <td class="table-td text-right">
                    <div class="flex items-center justify-end gap-1 flex-wrap">
                      <button @click="performAction('start', c.id, c.name)" :disabled="c.state === 'running'"
                        class="p-1.5 rounded text-green-600 hover:bg-green-50 dark:text-green-400 dark:hover:bg-green-900/30 disabled:opacity-30 disabled:cursor-not-allowed" title="Start">
                        <Play class="w-3.5 h-3.5" />
                      </button>
                      <button @click="performAction('stop', c.id, c.name)" :disabled="c.state !== 'running'"
                        class="p-1.5 rounded text-amber-600 hover:bg-amber-50 dark:text-amber-400 dark:hover:bg-amber-900/30 disabled:opacity-30 disabled:cursor-not-allowed" title="Stop">
                        <Square class="w-3.5 h-3.5" />
                      </button>
                      <button @click="performAction('restart', c.id, c.name)"
                        class="btn-icon-primary" title="Restart">
                        <RefreshCw class="w-3.5 h-3.5" />
                      </button>
                      <button @click="viewLogs(c.id, c.name)"
                        class="p-1.5 rounded text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-700" title="Logs">
                        <Terminal class="w-3.5 h-3.5" />
                      </button>
                      <button @click="viewInspect(c.id, c.name)"
                        class="p-1.5 rounded text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-700" title="Inspect">
                        <ZoomIn class="w-3.5 h-3.5" />
                      </button>
                      <button @click="performAction('rm', c.id, c.name)"
                        class="p-1.5 rounded text-red-500 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/30" title="Delete">
                        <Trash2 class="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </td>
                </tr>
                <tr v-if="filteredContainers.length === 0">
                  <td colspan="6" class="text-center p-8 text-slate-400 text-sm">
                    {{ containers.length === 0 ? 'No containers found. Deploy one from the Deploy tab.' : 'No containers match your filter.' }}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- ── Tab: Compose Projects ──────────────────────────── -->
        <div v-else-if="activeTab === 'compose'" class="space-y-3">
          <div v-if="!runtime?.available" class="text-center py-8 text-slate-400 text-sm">
            <AlertCircle class="w-8 h-8 mx-auto mb-2 opacity-40" />
            No container runtime available
          </div>
          <div v-else-if="!runtime?.compose_available" class="text-center py-8 text-amber-500 text-sm">
            <AlertCircle class="w-8 h-8 mx-auto mb-2 opacity-60" />
            No compose tool found.<br>
            <span class="text-xs text-slate-400 mt-1 block">Install podman-compose or docker compose plugin.</span>
          </div>
          <div v-else-if="isLoadingCompose && composeProjects.length === 0" class="flex justify-center py-8">
            <Loader2 class="w-6 h-6 animate-spin text-brand-500" />
          </div>
          <div v-else class="space-y-2">
            <!-- Project cards -->
            <div v-for="project in composeProjects" :key="project.name"
              class="border rounded-lg overflow-hidden transition-colors"
              :class="project.status === 'running' ? 'border-green-200 dark:border-green-800' : project.status === 'partial' ? 'border-amber-200 dark:border-amber-800' : 'border-slate-200 dark:border-slate-700'"
            >
              <!-- Project header -->
              <div class="flex items-center gap-3 px-4 py-3 cursor-pointer transition-colors"
                :class="isDark ? 'bg-slate-800/50 hover:bg-slate-800' : 'bg-slate-50 hover:bg-slate-100'"
                @click="toggleProject(project.name)">
                <component :is="expandedProjects.has(project.name) ? ChevronDown : ChevronRight" class="w-4 h-4 flex-shrink-0" :class="isDark ? 'text-slate-500' : 'text-slate-400'" />
                <Layers class="w-4 h-4 flex-shrink-0" :class="isDark ? 'text-brand-400' : 'text-brand-500'" />
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 flex-wrap">
                    <span class="font-semibold" :class="isDark ? 'text-slate-200' : 'text-slate-800'">{{ project.name }}</span>
                    <span class="px-2 py-0.5 rounded-full text-[10px] font-bold border" :class="projectStatusColor(project.status)">
                      {{ project.status.toUpperCase() }}
                    </span>
                    <span class="text-[10px] px-1.5 py-0.5 rounded" :class="isDark ? 'text-slate-400 bg-slate-700' : 'text-slate-400 bg-slate-200'">{{ project.source }}</span>
                  </div>
                  <div class="text-xs mt-0.5" :class="isDark ? 'text-slate-500' : 'text-slate-400'">{{ project.services.length }} service(s)</div>
                </div>
                <!-- Project actions -->
                <div class="flex items-center gap-1 flex-shrink-0" @click.stop>
                  <button @click="composeAction('restart', project.name)" class="btn-icon-primary" title="Restart">
                    <RefreshCw class="w-3.5 h-3.5" />
                  </button>
                  <button v-if="project.source === 'managed'" @click="composeAction('rebuild', project.name)" class="p-1.5 rounded" :class="isDark ? 'text-indigo-400 hover:bg-indigo-900/30' : 'text-indigo-600 hover:bg-indigo-50'" title="Rebuild (force recreate)">
                    <RotateCcw class="w-3.5 h-3.5" />
                  </button>
                  <button @click="viewComposeLogs(project.name)" class="p-1.5 rounded" :class="isDark ? 'text-slate-400 hover:bg-slate-700' : 'text-slate-600 hover:bg-slate-100'" title="View Logs">
                    <Terminal class="w-3.5 h-3.5" />
                  </button>
                  <button v-if="project.source === 'managed'" @click="openYamlModal(project.name)" class="p-1.5 rounded" :class="isDark ? 'text-slate-400 hover:bg-slate-700' : 'text-slate-600 hover:bg-slate-100'" title="View/Edit YAML">
                    <FileText class="w-3.5 h-3.5" />
                  </button>
                  <button @click="composeAction('stop', project.name)" class="p-1.5 rounded" :class="isDark ? 'text-amber-400 hover:bg-amber-900/30' : 'text-amber-600 hover:bg-amber-50'" title="Stop (down)">
                    <Square class="w-3.5 h-3.5" />
                  </button>
                  <button @click="deleteCompose(project.name)" class="p-1.5 rounded" :class="isDark ? 'text-red-400 hover:bg-red-900/30' : 'text-red-500 hover:bg-red-50'" title="Delete Project">
                    <Trash2 class="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>

              <!-- Services expanded -->
              <div v-if="expandedProjects.has(project.name)" class="divide-y" :class="isDark ? 'divide-slate-700' : 'divide-slate-100'">
                <div v-if="project.services.length === 0" class="px-4 py-3 text-sm italic" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
                  No running services detected.
                </div>
                <div v-for="svc in project.services" :key="svc.name"
                  class="px-4 py-2.5 flex items-center gap-3" :class="isDark ? 'hover:bg-slate-800/50' : 'hover:bg-slate-50'">
                  <div class="w-2 h-2 rounded-full flex-shrink-0" :class="svc.state === 'running' ? 'bg-green-500' : 'bg-slate-300'"></div>
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2">
                      <span class="text-sm font-medium" :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ svc.name }}</span>
                      <span class="text-[10px] px-1.5 py-0.5 rounded font-semibold" :class="stateColor(svc.state)">{{ svc.state }}</span>
                    </div>
                    <div class="text-xs font-mono truncate" :class="isDark ? 'text-slate-500' : 'text-slate-400'">{{ svc.image }}</div>
                    <div v-if="svc.ports.length" class="flex gap-1 mt-0.5 flex-wrap">
                      <span v-for="p in svc.ports" :key="p" class="text-[10px] font-mono px-1 rounded" :class="isDark ? 'bg-slate-700 text-slate-300' : 'bg-slate-100 text-slate-700'">{{ p }}</span>
                    </div>
                  </div>
                  <!-- Service actions -->
                  <div class="flex items-center gap-1 flex-shrink-0">
                    <button @click="viewComposeLogs(project.name, svc.name)" class="p-1 rounded" :class="isDark ? 'text-slate-400 hover:bg-slate-700' : 'text-slate-500 hover:bg-slate-100'" title="Service Logs">
                      <Terminal class="w-3.5 h-3.5" />
                    </button>
                    <button v-if="project.source === 'managed'" @click="openScaleModal(project.name, svc.name, 1)" class="p-1 rounded" :class="isDark ? 'text-slate-400 hover:bg-slate-700' : 'text-slate-500 hover:bg-slate-100'" title="Scale">
                      <Scaling class="w-3.5 h-3.5" />
                    </button>
                    <button v-if="svc.container_id" @click="viewLogs(svc.container_id, svc.name)" class="p-1 rounded" :class="isDark ? 'text-slate-400 hover:bg-slate-700' : 'text-slate-500 hover:bg-slate-100'" title="Container Logs">
                      <Eye class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <div v-if="composeProjects.length === 0" class="text-center py-8 text-sm" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
              No compose projects found. Deploy one from the Deploy tab.
            </div>
          </div>
        </div>

        <!-- ── Tab: Deploy ─────────────────────────────────────── -->
        <div v-else-if="activeTab === 'deploy'" class="space-y-4">
          <!-- Sub-tabs -->
          <div class="flex gap-1 border-b" :class="isDark ? 'border-slate-700' : 'border-slate-200'">
            <button v-for="t in [{ id: 'container', label: 'Single Container', icon: Box }, { id: 'compose', label: 'Compose (YAML)', icon: Layers }]"
              :key="t.id" @click="deployTab = t.id"
              class="flex items-center gap-1.5 px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors"
              :class="deployTab === t.id ? (isDark ? 'border-brand-400 text-brand-300' : 'border-brand-500 text-brand-700') : (isDark ? 'border-transparent text-slate-400 hover:text-slate-200' : 'border-transparent text-slate-500 hover:text-slate-700')"
            >
              <component :is="t.icon" class="w-4 h-4" />{{ t.label }}
            </button>
          </div>

          <!-- Deploy Container form -->
          <div v-if="deployTab === 'container'" class="space-y-4 max-w-2xl">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div>
                <label class="text-xs font-semibold uppercase tracking-wider mb-1 block" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Container Name *</label>
                <input v-model="dcName" type="text" placeholder="e.g. my-nginx" class="input-field" />
              </div>
              <div>
                <label class="text-xs font-semibold uppercase tracking-wider mb-1 block" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Image *</label>
                <input v-model="dcImage" type="text" placeholder="e.g. nginx:alpine" class="input-field" />
              </div>
            </div>
            <div>
              <label class="text-xs font-semibold uppercase tracking-wider mb-1 block" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Ports <span class="font-normal" :class="isDark ? 'text-slate-500' : 'text-slate-400'">(comma-separated: 8080:80, 443:443)</span></label>
              <input v-model="dcPorts" type="text" placeholder="8080:80, 3000:3000" class="input-field" />
            </div>
            <div>
              <label class="text-xs font-semibold uppercase tracking-wider mb-1 block" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Environment Variables <span class="font-normal" :class="isDark ? 'text-slate-500' : 'text-slate-400'">(one per line: KEY=value)</span></label>
              <textarea v-model="dcEnv" rows="3" placeholder="NODE_ENV=production&#10;PORT=3000" class="input-field font-mono text-xs resize-none"></textarea>
            </div>
            <div>
              <label class="text-xs font-semibold uppercase tracking-wider mb-1 block" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Volumes <span class="font-normal" :class="isDark ? 'text-slate-500' : 'text-slate-400'">(one per line: /host:/container)</span></label>
              <textarea v-model="dcVolumes" rows="2" placeholder="/data:/app/data&#10;/config:/etc/app" class="input-field font-mono text-xs resize-none"></textarea>
            </div>
            <div>
              <label class="text-xs font-semibold uppercase tracking-wider mb-1 block" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Restart Policy</label>
              <select v-model="dcRestart" class="input-field">
                <option value="">None (default)</option>
                <option value="always">always</option>
                <option value="unless-stopped">unless-stopped</option>
                <option value="on-failure">on-failure</option>
              </select>
            </div>
            <button @click="deployContainer" class="btn-primary" :disabled="isDeployingContainer">
              <Loader2 v-if="isDeployingContainer" class="w-4 h-4 animate-spin" />
              <Play v-else class="w-4 h-4" />
              {{ isDeployingContainer ? 'Deploying...' : 'Deploy Container' }}
            </button>
          </div>

          <!-- Deploy Compose form -->
          <div v-else class="space-y-4 max-w-2xl">
            <div>
              <label class="text-xs font-semibold uppercase tracking-wider mb-1 block" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Project Name *</label>
              <input v-model="cpName" type="text" placeholder="e.g. my-stack" class="input-field max-w-xs" />
              <p class="text-xs mt-1" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Letters, numbers, dashes, underscores only.</p>
            </div>
            <div>
              <label class="text-xs font-semibold uppercase tracking-wider mb-1 block" :class="isDark ? 'text-slate-400' : 'text-slate-500'">docker-compose.yml Content *</label>
              <textarea
                v-model="cpYaml"
                rows="18"
                class="input-field font-mono text-xs resize-y w-full"
                placeholder="services:
  app:
    image: nginx:alpine
    ports:
      - &quot;8080:80&quot;"
              ></textarea>
            </div>
            <button @click="deployCompose" class="btn-primary" :disabled="isDeployingCompose || !runtime?.compose_available">
              <Loader2 v-if="isDeployingCompose" class="w-4 h-4 animate-spin" />
              <Layers v-else class="w-4 h-4" />
              {{ isDeployingCompose ? 'Deploying...' : 'Deploy Compose Stack' }}
            </button>
            <p v-if="!runtime?.compose_available" class="text-xs text-amber-600 dark:text-amber-500">
              Compose is not available. Install podman-compose or docker compose plugin.
            </p>
          </div>
        </div>

        <!-- ── Tab: VMs (Podman nested containers) ────────────────── -->
        <div v-else-if="activeTab === 'vms'" class="space-y-4">

          <!-- Header / Deploy button -->
          <div class="flex items-center justify-between flex-wrap gap-3">
            <div>
              <h3 class="font-semibold text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-700'">
                OS Containers via Podman
              </h3>
              <p class="text-xs mt-0.5" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
                Deploy Ubuntu 24.04, Debian 12, atau Arch Linux sebagai nested Podman container dengan InfoIn Server backend.
              </p>
            </div>
            <div class="flex gap-2">
              <button @click="fetchVms" class="btn-secondary text-xs">
                <RefreshCw class="w-3.5 h-3.5" /> Refresh
              </button>
              <button @click="openVmDeploy" class="btn-primary text-xs" :disabled="!runtime?.available">
                <Plus class="w-3.5 h-3.5" /> Deploy New VM
              </button>
            </div>
          </div>

          <!-- No runtime warning -->
          <div v-if="!runtime?.available" class="rounded-lg border p-3 text-xs flex items-center gap-2"
            :class="isDark ? 'border-red-800 bg-red-900/20 text-red-400' : 'border-red-200 bg-red-50 text-red-600'">
            <AlertCircle class="w-4 h-4 flex-shrink-0" />
            Podman tidak terdeteksi di server ini. Fitur VM membutuhkan Podman.
          </div>

          <!-- Loading -->
          <div v-if="isLoadingVms && vms.length === 0" class="flex justify-center py-12">
            <Loader2 class="w-6 h-6 animate-spin text-brand-500" />
          </div>

          <!-- Empty state -->
          <div v-else-if="vms.length === 0" class="text-center py-12 border-2 border-dashed rounded-xl"
            :class="isDark ? 'border-slate-700 text-slate-500' : 'border-slate-200 text-slate-400'">
            <Server class="w-10 h-10 mx-auto mb-3 opacity-30" />
            <p class="font-medium text-sm">Belum ada VM yang di-deploy</p>
            <p class="text-xs mt-1 opacity-70">Klik "Deploy New VM" untuk membuat container Ubuntu, Debian, atau Arch.</p>
          </div>

          <!-- VM grid -->
          <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
            <div v-for="vm in vms" :key="vm.id"
              class="border rounded-xl overflow-hidden transition-shadow hover:shadow-md"
              :class="isDark ? 'bg-slate-800/60 border-slate-700' : 'bg-white border-slate-200'">

              <!-- Card header -->
              <div class="px-4 pt-4 pb-3 flex items-start justify-between gap-2">
                <div class="flex items-center gap-2 min-w-0">
                  <div class="p-2 rounded-lg flex-shrink-0"
                    :class="isDark ? 'bg-slate-700' : 'bg-slate-100'">
                    <HardDrive class="w-4 h-4" :class="isDark ? 'text-slate-300' : 'text-slate-600'" />
                  </div>
                  <div class="min-w-0">
                    <p class="font-semibold truncate text-sm" :class="isDark ? 'text-slate-100' : 'text-slate-800'">{{ vm.name }}</p>
                    <span class="inline-flex items-center text-[10px] font-medium px-1.5 py-0.5 rounded border mt-0.5" :class="distroColor(vm.distro)">
                      {{ distroLabel(vm.distro) }}
                    </span>
                  </div>
                </div>
                <!-- State badge -->
                <span class="inline-flex items-center gap-1 text-[10px] font-semibold px-2 py-1 rounded-full flex-shrink-0" :class="vmStateColor(vm.state)">
                  <span class="relative flex h-1.5 w-1.5">
                    <span v-if="vm.state === 'running'" class="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                    <span class="relative inline-flex rounded-full h-1.5 w-1.5" :class="vm.state === 'running' ? 'bg-green-500' : 'bg-slate-400'"></span>
                  </span>
                  {{ vm.state || 'exited' }}
                </span>
              </div>

              <!-- Card info -->
              <div class="px-4 pb-3 space-y-1.5">
                <div class="flex items-center gap-1.5 text-xs" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
                  <Cpu class="w-3.5 h-3.5 flex-shrink-0" />
                  <span>User: <strong class="font-medium" :class="isDark ? 'text-slate-200' : 'text-slate-700'">{{ vm.username }}</strong></span>
                </div>
                <div class="flex items-center gap-1.5 text-xs" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
                  <Link class="w-3.5 h-3.5 flex-shrink-0" />
                  <span class="font-mono truncate">{{ vm.host_ip }}:<strong>{{ vm.backend_port }}</strong></span>
                </div>
                <div v-if="vm.container_id" class="flex items-center gap-1.5 text-xs" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
                  <Box class="w-3.5 h-3.5 flex-shrink-0" />
                  <span class="font-mono truncate" :title="vm.container_id">{{ vm.container_id }}</span>
                </div>
                <div class="text-[10px]" :class="isDark ? 'text-slate-600' : 'text-slate-400'">
                  Created: {{ new Date(vm.created_at).toLocaleDateString() }}
                </div>
              </div>

              <!-- Card actions -->
              <div class="px-4 pb-4 flex items-center gap-1.5 flex-wrap border-t pt-3"
                :class="isDark ? 'border-slate-700' : 'border-slate-100'">
                <button @click="vmAction('start', vm.name)" :disabled="vm.state === 'running'"
                  class="inline-flex items-center gap-1 px-2.5 py-1 rounded text-xs font-semibold
                         bg-green-100 text-green-800 hover:bg-green-200 dark:bg-green-900/40 dark:text-green-300 dark:hover:bg-green-900/70
                         disabled:opacity-30 disabled:cursor-not-allowed transition-colors">
                  <Play class="w-3 h-3" /> Start
                </button>
                <button @click="vmAction('stop', vm.name)" :disabled="vm.state !== 'running'"
                  class="inline-flex items-center gap-1 px-2.5 py-1 rounded text-xs font-semibold
                         bg-amber-100 text-amber-800 hover:bg-amber-200 dark:bg-amber-900/40 dark:text-amber-300 dark:hover:bg-amber-900/70
                         disabled:opacity-30 disabled:cursor-not-allowed transition-colors">
                  <Square class="w-3 h-3" /> Stop
                </button>
                <button @click="addVmToDashboard(vm)"
                  class="inline-flex items-center gap-1 px-2.5 py-1 rounded text-xs font-semibold
                         bg-indigo-500 text-white hover:bg-indigo-600 dark:bg-indigo-600 dark:hover:bg-indigo-500
                         shadow-sm transition-colors" title="Add backend to Dashboard">
                  <ExternalLink class="w-3 h-3" /> Add to Dashboard
                </button>
                <button @click="deleteVm(vm.name)"
                  class="inline-flex items-center gap-1 px-2.5 py-1 rounded text-xs font-semibold ml-auto
                         bg-red-100 text-red-700 hover:bg-red-200 dark:bg-red-900/40 dark:text-red-300 dark:hover:bg-red-900/70
                         transition-colors">
                  <Trash2 class="w-3 h-3" /> Delete
                </button>
              </div>
            </div>
          </div>

          <!-- Requirements note -->
          <div class="rounded-lg border p-3 text-xs space-y-1"
            :class="isDark ? 'border-slate-700 bg-slate-800/30 text-slate-400' : 'border-slate-200 bg-slate-50 text-slate-500'">
            <p class="font-semibold" :class="isDark ? 'text-slate-300' : 'text-slate-600'">Cara kerja</p>
            <ul class="list-disc list-inside space-y-0.5">
              <li>Setiap VM adalah <strong>Podman container</strong> yang menjalankan distro Linux lengkap</li>
              <li>Binary InfoIn Server di-bind-mount dari host → container (tidak perlu build ulang)</li>
              <li>Port backend di-forward dari container ke host via <code class="font-mono bg-slate-200 dark:bg-slate-700 px-1 rounded">-p PORT:8080</code></li>
              <li>Debian 12 menggunakan <code class="font-mono bg-slate-200 dark:bg-slate-700 px-1 rounded">ubuntu:24.04</code> base (kompatibilitas GLIBC)</li>
              <li>Data VM disimpan di Podman named volume (<code class="font-mono bg-slate-200 dark:bg-slate-700 px-1 rounded">infoinserver-vm-&lt;name&gt;</code>)</li>
            </ul>
          </div>
        </div>

      </div>
    </div>

    <!-- ── Logs Modal ────────────────────────────────────────────── -->
    <Teleport to="body">
      <div v-if="logsModal.open" class="fixed inset-0 bg-black/60 z-50 flex items-center justify-center p-4" @click.self="logsModal.open = false">
        <div class="bg-white dark:bg-slate-800 rounded-xl w-full max-w-4xl max-h-[80vh] flex flex-col" style="box-shadow: var(--shadow-modal)">
          <div class="flex items-center justify-between px-4 py-3 border-b border-slate-200 dark:border-slate-700">
            <h3 class="font-semibold text-slate-800 dark:text-slate-100 flex items-center gap-2"><Terminal class="w-4 h-4 text-brand-500" />{{ logsModal.title }}</h3>
            <button @click="logsModal.open = false" class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 dark:text-slate-400"><X class="w-4 h-4" /></button>
          </div>
          <div class="flex-1 overflow-auto p-3 bg-slate-950 rounded-b-xl">
            <div v-if="logsModal.loading" class="flex items-center justify-center h-32 text-slate-400">
              <Loader2 class="w-6 h-6 animate-spin" />
            </div>
            <pre v-else class="text-xs text-green-400 font-mono whitespace-pre-wrap leading-relaxed">{{ logsModal.logs }}</pre>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- ── Inspect Modal ─────────────────────────────────────────── -->
    <Teleport to="body">
      <div v-if="inspectModal.open" class="fixed inset-0 bg-black/60 z-50 flex items-center justify-center p-4" @click.self="inspectModal.open = false">
        <div class="bg-white dark:bg-slate-800 rounded-xl w-full max-w-4xl max-h-[80vh] flex flex-col" style="box-shadow: var(--shadow-modal)">
          <div class="flex items-center justify-between px-4 py-3 border-b border-slate-200 dark:border-slate-700">
            <h3 class="font-semibold text-slate-800 dark:text-slate-100 flex items-center gap-2"><ZoomIn class="w-4 h-4 text-brand-500" />{{ inspectModal.title }}</h3>
            <button @click="inspectModal.open = false" class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 dark:text-slate-400"><X class="w-4 h-4" /></button>
          </div>
          <div class="flex-1 overflow-auto p-3 bg-slate-950 rounded-b-xl">
            <pre class="text-xs text-slate-200 font-mono whitespace-pre-wrap leading-relaxed">{{ JSON.stringify(inspectModal.data, null, 2) }}</pre>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- ── YAML Modal ────────────────────────────────────────────── -->
    <Teleport to="body">
      <div v-if="yamlModal.open" class="fixed inset-0 bg-black/60 z-50 flex items-center justify-center p-4" @click.self="yamlModal.open = false">
        <div class="bg-white dark:bg-slate-800 rounded-xl w-full max-w-4xl max-h-[85vh] flex flex-col" style="box-shadow: var(--shadow-modal)">
          <div class="flex items-center justify-between px-4 py-3 border-b border-slate-200 dark:border-slate-700">
            <h3 class="font-semibold text-slate-800 dark:text-slate-100 flex items-center gap-2">
              <FileText class="w-4 h-4 text-brand-500" />
              {{ yamlModal.projectName }} — docker-compose.yml
            </h3>
            <div class="flex items-center gap-2">
              <button v-if="!yamlModal.editing" @click="yamlModal.editing = true" class="btn-secondary text-xs"><Pencil class="w-3.5 h-3.5" /> Edit</button>
              <button v-if="yamlModal.editing" @click="saveYaml" class="btn-primary text-xs" :disabled="yamlModal.saving">
                <Loader2 v-if="yamlModal.saving" class="w-3.5 h-3.5 animate-spin" />
                <Save v-else class="w-3.5 h-3.5" />
                {{ yamlModal.saving ? 'Saving...' : 'Save & Apply' }}
              </button>
              <button v-if="yamlModal.editing" @click="yamlModal.editing = false" class="btn-secondary text-xs"><X class="w-3.5 h-3.5" /> Cancel</button>
              <button @click="yamlModal.open = false" class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 dark:text-slate-400"><X class="w-4 h-4" /></button>
            </div>
          </div>
          <div class="flex-1 overflow-auto p-3 bg-slate-950 rounded-b-xl">
            <textarea v-if="yamlModal.editing"
              v-model="yamlModal.yaml"
              class="w-full h-full min-h-[400px] bg-transparent text-xs text-slate-100 font-mono p-1 resize-none outline-none"
            ></textarea>
            <pre v-else class="text-xs text-slate-200 font-mono whitespace-pre-wrap leading-relaxed">{{ yamlModal.yaml }}</pre>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- ── Scale Modal ───────────────────────────────────────────── -->
    <Teleport to="body">
      <div v-if="scaleModal.open" class="fixed inset-0 z-50 flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-black/60'" @click.self="scaleModal.open = false">
        <div class="rounded-xl w-full max-w-sm p-5 space-y-4" :class="isDark ? 'bg-slate-800' : 'bg-white'" style="box-shadow: var(--shadow-modal)">
          <h3 class="font-semibold flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'"><Scaling class="w-4 h-4 text-brand-500" /> Scale Service</h3>
          <div>
            <p class="text-sm" :class="isDark ? 'text-slate-300' : 'text-slate-600'">Service: <strong>{{ scaleModal.service }}</strong></p>
            <p class="text-xs" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Project: {{ scaleModal.projectName }}</p>
          </div>
          <div>
            <label class="text-xs font-semibold uppercase mb-1 block" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Replicas</label>
            <input v-model.number="scaleModal.count" type="number" min="0" max="20" class="input-field w-32" />
          </div>
          <div class="flex gap-2 justify-end">
            <button @click="scaleModal.open = false" class="btn-secondary text-sm">Cancel</button>
            <button @click="doScale" class="btn-primary text-sm"><Scaling class="w-4 h-4" /> Scale</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- ── VM Deploy Modal ────────────────────────────────────────── -->
    <Teleport to="body">
      <div v-if="vmDeployModal.open" class="fixed inset-0 z-50 flex items-center justify-center p-4"
        :class="isDark ? 'bg-slate-950/80' : 'bg-black/60'"
        @click.self="vmDeployModal.status !== 'running' && closeVmDeploy()">
        <div class="rounded-xl w-full max-w-2xl flex flex-col max-h-[90vh]"
          :class="isDark ? 'bg-slate-800' : 'bg-white'" style="box-shadow: var(--shadow-modal)">

          <!-- Modal header -->
          <div class="flex items-center justify-between px-5 py-4 border-b"
            :class="isDark ? 'border-slate-700' : 'border-slate-200'">
            <h3 class="font-semibold flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'">
              <Server class="w-4 h-4 text-brand-500" />
              Deploy New VM
            </h3>
            <button v-if="vmDeployModal.status !== 'running'" @click="closeVmDeploy"
              class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 dark:text-slate-400">
              <X class="w-4 h-4" />
            </button>
          </div>

          <div class="flex-1 overflow-y-auto">
            <!-- Form (idle state) -->
            <div v-if="vmDeployModal.status === 'idle'" class="p-5 space-y-4">

              <!-- Distro picker -->
              <div>
                <label class="text-xs font-semibold uppercase tracking-wider mb-2 block"
                  :class="isDark ? 'text-slate-400' : 'text-slate-500'">Pilih Distro *</label>
                <div class="grid grid-cols-2 gap-3">
                  <button v-for="d in [
                    { id: 'ubuntu-2404', label: 'Ubuntu 24.04', desc: 'LTS, direkomendasikan', disabled: false },
                    { id: 'arch',        label: 'Arch Linux',   desc: 'Coming soon',           disabled: true  },
                  ]" :key="d.id"
                    @click="!d.disabled && (vmForm.distro = d.id)"
                    :disabled="d.disabled"
                    class="flex flex-col items-center gap-1 p-3 rounded-lg border text-center text-xs transition-all relative"
                    :class="d.disabled
                      ? (isDark ? 'border-slate-700 bg-slate-800/30 text-slate-600 cursor-not-allowed' : 'border-slate-200 bg-slate-50 text-slate-400 cursor-not-allowed')
                      : vmForm.distro === d.id
                        ? (isDark ? 'border-brand-500 bg-brand-900/30 text-brand-300' : 'border-brand-500 bg-brand-50 text-brand-700')
                        : (isDark ? 'border-slate-600 text-slate-400 hover:border-slate-500' : 'border-slate-200 text-slate-500 hover:border-slate-300')">
                    <HardDrive class="w-5 h-5" :class="d.disabled ? 'opacity-40' : ''" />
                    <span class="font-semibold" :class="d.disabled ? 'opacity-40' : ''">{{ d.label }}</span>
                    <span class="text-[10px]" :class="d.disabled ? 'text-amber-500 dark:text-amber-400 font-medium' : 'opacity-70'">{{ d.desc }}</span>
                  </button>
                </div>
              </div>

              <!-- VM name + host IP -->
              <div class="grid grid-cols-2 gap-3">
                <div>
                  <label class="text-xs font-semibold uppercase tracking-wider mb-1 block"
                    :class="isDark ? 'text-slate-400' : 'text-slate-500'">VM Name *</label>
                  <input v-model="vmForm.name" type="text" placeholder="e.g. my-ubuntu" class="input-field" />
                  <p class="text-[10px] mt-1" :class="isDark ? 'text-slate-600' : 'text-slate-400'">letters, numbers, dashes, underscores</p>
                </div>
                <div>
                  <label class="text-xs font-semibold uppercase tracking-wider mb-1 block"
                    :class="isDark ? 'text-slate-400' : 'text-slate-500'">Host IP *</label>
                  <input v-model="vmForm.host_ip" type="text" placeholder="e.g. 192.168.1.10" class="input-field" />
                  <p class="text-[10px] mt-1" :class="isDark ? 'text-slate-600' : 'text-slate-400'">used for dashboard URL</p>
                </div>
              </div>

              <!-- User + password -->
              <div class="grid grid-cols-2 gap-3">
                <div>
                  <label class="text-xs font-semibold uppercase tracking-wider mb-1 block"
                    :class="isDark ? 'text-slate-400' : 'text-slate-500'">Username *</label>
                  <input v-model="vmForm.username" type="text" placeholder="e.g. admin" class="input-field" />
                </div>
                <div>
                  <label class="text-xs font-semibold uppercase tracking-wider mb-1 block"
                    :class="isDark ? 'text-slate-400' : 'text-slate-500'">Password *</label>
                  <input v-model="vmForm.password" type="password" placeholder="for root + user" class="input-field" />
                </div>
              </div>

              <!-- Backend port -->
              <div class="max-w-[200px]">
                <label class="text-xs font-semibold uppercase tracking-wider mb-1 block"
                  :class="isDark ? 'text-slate-400' : 'text-slate-500'">Backend Port *</label>
                <input v-model.number="vmForm.backend_port" type="number" min="1024" max="65535" class="input-field" />
                <p class="text-[10px] mt-1" :class="isDark ? 'text-slate-600' : 'text-slate-400'">InfoIn Server backend port (forwarded from VM)</p>
              </div>

              <!-- Info box -->
              <div class="rounded-lg p-3 text-xs space-y-1"
                :class="isDark ? 'bg-slate-700/50 text-slate-400' : 'bg-blue-50 text-blue-600'">
                <p class="font-semibold">Yang akan di-deploy:</p>
                <ul class="list-disc list-inside space-y-0.5">
                  <li>Podman container dari image <code class="font-mono">{{ vmForm.distro === 'arch' ? 'archlinux:base' : 'ubuntu:24.04' }}</code></li>
                  <li>User: <code class="font-mono">root</code> + <code class="font-mono">{{ vmForm.username || 'user' }}</code> (dengan sudo)</li>
                  <li>InfoIn Server backend berjalan di port <strong>{{ vmForm.backend_port }}</strong></li>
                  <li>Port {{ vmForm.backend_port }} di-forward dari container ke host</li>
                  <li>Data disimpan di Podman volume <code class="font-mono">infoinserver-vm-{{ vmForm.name || 'nama' }}</code></li>
                </ul>
              </div>

              <div class="flex justify-end gap-2 pt-1">
                <button @click="closeVmDeploy" class="btn-secondary">Cancel</button>
                <button @click="deployVm" class="btn-primary">
                  <Server class="w-4 h-4" /> Start Deployment
                </button>
              </div>
            </div>

            <!-- Progress terminal (running/done/error) -->
            <div v-else class="flex flex-col h-full">
              <div class="px-5 py-3 border-b flex items-center gap-2"
                :class="isDark ? 'border-slate-700' : 'border-slate-200'">
                <Loader2 v-if="vmDeployModal.status === 'running'" class="w-4 h-4 animate-spin text-brand-500" />
                <CheckCircle2 v-else-if="vmDeployModal.status === 'done'" class="w-4 h-4 text-green-500" />
                <AlertCircle v-else class="w-4 h-4 text-red-500" />
                <span class="text-sm font-medium" :class="isDark ? 'text-slate-200' : 'text-slate-700'">
                  <span v-if="vmDeployModal.status === 'running'">Deploying VM — please wait...</span>
                  <span v-else-if="vmDeployModal.status === 'done'" class="text-green-600 dark:text-green-400">VM deployed successfully!</span>
                  <span v-else class="text-red-600 dark:text-red-400">Deployment failed</span>
                </span>
              </div>

              <!-- Log terminal -->
              <div id="vm-deploy-log"
                class="flex-1 overflow-y-auto p-4 bg-slate-950 font-mono text-xs leading-relaxed space-y-0.5"
                style="min-height: 280px; max-height: 400px">
                <div v-for="(entry, i) in vmDeployModal.logs" :key="i"
                  :class="{
                    'text-green-400': entry.event === 'done',
                    'text-red-400': entry.event === 'error',
                    'text-amber-300 font-semibold': entry.event === 'step',
                    'text-slate-400': entry.event === 'log',
                    'text-blue-400': entry.event === 'info',
                  }">
                  <span v-if="entry.event === 'step'">▶ </span>
                  <span v-else-if="entry.event === 'done'">✓ </span>
                  <span v-else-if="entry.event === 'error'">✗ </span>
                  <span v-else class="opacity-50">  </span>
                  {{ entry.msg }}
                </div>
                <div v-if="vmDeployModal.status === 'running'" class="text-slate-600 animate-pulse">█</div>
              </div>

              <!-- Success result -->
              <div v-if="vmDeployModal.status === 'done' && vmDeployModal.result"
                class="px-5 py-4 border-t space-y-3"
                :class="isDark ? 'border-slate-700 bg-slate-800/50' : 'border-slate-200 bg-green-50'">
                <p class="text-sm font-semibold" :class="isDark ? 'text-green-400' : 'text-green-700'">VM Ready</p>
                <div class="grid grid-cols-2 gap-2 text-xs">
                  <div :class="isDark ? 'text-slate-400' : 'text-slate-500'">VM Name</div>
                  <div class="font-medium font-mono" :class="isDark ? 'text-slate-200' : 'text-slate-700'">{{ vmDeployModal.result.name }}</div>
                  <div :class="isDark ? 'text-slate-400' : 'text-slate-500'">Distro</div>
                  <div class="font-medium" :class="isDark ? 'text-slate-200' : 'text-slate-700'">{{ distroLabel(vmDeployModal.result.distro) }}</div>
                  <div :class="isDark ? 'text-slate-400' : 'text-slate-500'">Backend URL</div>
                  <div class="font-mono font-medium" :class="isDark ? 'text-brand-300' : 'text-brand-600'">{{ vmDeployModal.result.dashboard_url }}</div>
                </div>
                <div class="flex gap-2 pt-1">
                  <button @click="addVmToDashboard(vmDeployModal.result); closeVmDeploy()" class="btn-primary text-sm">
                    <ExternalLink class="w-3.5 h-3.5" /> Add to Dashboard
                  </button>
                  <button @click="closeVmDeploy" class="btn-secondary text-sm">Close</button>
                </div>
              </div>

              <!-- Error actions -->
              <div v-else-if="vmDeployModal.status === 'error'"
                class="px-5 py-4 border-t flex gap-2"
                :class="isDark ? 'border-slate-700' : 'border-slate-200'">
                <button @click="vmDeployModal.status = 'idle'" class="btn-secondary text-sm">
                  <RefreshCw class="w-3.5 h-3.5" /> Try Again
                </button>
                <button @click="closeVmDeploy" class="btn-secondary text-sm">Close</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

  </div>
</template>
