<script setup>
import { ref, onMounted } from 'vue'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { Cloud, Save, Route, Plus, Trash2, Loader2, KeyRound } from 'lucide-vue-next'

const { getActiveServerUrl } = useServerStore()
const { showToast, showConfirm } = useToastStore()

const config = ref({ account_id: '', tunnel_id: '', api_token: '' })
const isLoadingConfig = ref(true)

const routes = ref([])
const isLoadingRoutes = ref(false)

const newHostname = ref('')
const newService = ref('')

const fetchConfig = async () => {
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/cloudflare/api/config`)
    if(res.ok) {
      const data = await res.json()
      if(data) config.value = data
    }
  } catch(e) {}
  isLoadingConfig.value = false
}

const saveConfig = async () => {
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/cloudflare/api/config`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config.value)
    })
    const data = await res.json()
    if(res.ok) showToast("Success", data.message, "success")
    else showToast("Error", data, "error")
  } catch(e) {
    showToast("Error", e.message, "error")
  }
}

const fetchRoutes = async () => {
  if (!config.value.account_id || !config.value.tunnel_id || !config.value.api_token) return
  isLoadingRoutes.value = true
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/cloudflare/api/routes`)
    const data = await res.json()
    if(res.ok) {
      const ingress = data?.result?.config?.ingress || []
      // filter out catch-all
      routes.value = ingress.filter(r => r.service !== 'http_status:404')
    } else {
      showToast("Cloudflare API Error", JSON.stringify(data), "error")
    }
  } catch(e) {
    showToast("Error", "Failed to fetch routes: " + e.message, "error")
  } finally {
    isLoadingRoutes.value = false
  }
}

const addRoute = async () => {
  if(!newHostname.value || !newService.value) {
    showToast("Warning", "Hostname and Service must be filled", "warning")
    return
  }
  
  showToast("Info", "Creating route in Cloudflare...")
  try {
    const res = await fetch(`${getActiveServerUrl()}/api/cloudflare/api/routes`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ hostname: newHostname.value, service: newService.value })
    })
    const data = await res.json()
    if(res.ok) {
      showToast("Success", data.message, "success")
      newHostname.value = ''
      newService.value = ''
      fetchRoutes()
    } else {
      showToast("Error", data, "error")
    }
  } catch(e) {
    showToast("Error", e.message, "error")
  }
}

const deleteRoute = (hostname) => {
  showConfirm("Hapus Route", `Yakin ingin menghapus route untuk domain ${hostname}?`, async () => {
    try {
      const res = await fetch(`${getActiveServerUrl()}/api/cloudflare/api/routes`, {
        method: 'DELETE',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ hostname, service: '' })
      })
      const data = await res.json()
      if(res.ok) {
        showToast("Success", "Route deleted", "success")
        fetchRoutes()
      } else {
        showToast("Error", data, "error")
      }
    } catch(e) {
      showToast("Error", e.message, "error")
    }
  })
}

onMounted(() => {
  fetchConfig().then(fetchRoutes)
})
</script>

<template>
  <div class="space-y-6">
    <!-- API Config Form -->
    <section class="card">
      <h2 class="card-title"><KeyRound class="w-5 h-5 text-brand-500" /> Cloudflare API Configuration</h2>
      <p class="text-sm text-slate-500 mb-4">Required to manage Zero Trust Tunnel routes programmatically.</p>
      
      <div v-if="isLoadingConfig" class="flex gap-2 items-center text-sm text-slate-500">
        <Loader2 class="w-4 h-4 animate-spin" /> Loading config...
      </div>
      <div v-else class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-4">
        <div>
          <label class="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1 block">Account ID</label>
          <input v-model="config.account_id" type="text" placeholder="e.g. 1a2b3c..." class="input-field font-mono text-sm">
        </div>
        <div>
          <label class="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1 block">Tunnel ID</label>
          <input v-model="config.tunnel_id" type="text" placeholder="e.g. d6f9f7..." class="input-field font-mono text-sm">
        </div>
        <div>
          <label class="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1 block">API Token</label>
          <input v-model="config.api_token" type="password" placeholder="Cloudflare API Token" class="input-field font-mono text-sm">
        </div>
      </div>
      <div class="flex gap-2">
        <button @click="saveConfig" class="btn-primary"><Save class="w-4 h-4" /> Save Config</button>
        <button @click="fetchRoutes" class="btn-outline" :disabled="!config.api_token"><RefreshCw class="w-4 h-4" /> Load Routes</button>
      </div>
    </section>

    <!-- Published Routes -->
    <section class="card">
      <h2 class="card-title"><Route class="w-5 h-5 text-brand-500" /> Published Application Routes</h2>
      <p class="text-sm text-slate-500 mb-6">Allow your Tunnel to reach applications whose domains you connected to Cloudflare.</p>
      
      <div class="bg-slate-50 p-4 rounded-lg border border-slate-200 mb-6">
        <h3 class="text-sm font-semibold mb-3">Add New Route</h3>
        <div class="flex flex-col sm:flex-row gap-3">
          <input v-model="newHostname" type="text" placeholder="Public Hostname (e.g. app.widy4aa.my.id)" class="input-field sm:w-1/2">
          <input v-model="newService" type="text" placeholder="Service URL (e.g. http://127.0.0.1:80)" class="input-field flex-1">
          <button @click="addRoute" class="btn-primary whitespace-nowrap"><Plus class="w-4 h-4" /> Add Route</button>
        </div>
      </div>

      <div v-if="isLoadingRoutes" class="flex justify-center p-8">
        <Loader2 class="w-8 h-8 animate-spin text-brand-500" />
      </div>
      
      <div v-else class="overflow-x-auto">
        <table class="w-full relative">
          <thead class="bg-white border-b-2 border-slate-200">
            <tr>
              <th class="table-th">Public Hostname</th>
              <th class="table-th">Service Origin</th>
              <th class="table-th text-right">Action</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="r in routes" :key="r.hostname" class="hover:bg-slate-50">
              <td class="table-td font-semibold text-brand-700">{{ r.hostname }}</td>
              <td class="table-td font-mono text-xs">{{ r.service }}</td>
              <td class="table-td text-right">
                <button @click="deleteRoute(r.hostname)" class="p-1.5 text-red-400 hover:text-red-600 hover:bg-red-50 rounded" title="Remove Route">
                  <Trash2 class="w-4 h-4" />
                </button>
              </td>
            </tr>
            <tr v-if="routes.length === 0">
              <td colspan="3" class="text-center p-6 text-slate-500">No routes published. Sync config first or add a new route.</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

  </div>
</template>