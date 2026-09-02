<script setup>
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { Settings, Plus, Trash2, Power, RefreshCw, Edit2, LogIn, Loader2, AlertCircle, User, Lock, Cloud, Shield, Ban } from 'lucide-vue-next'

const route = useRoute()
const router = useRouter()
const { apiFetch } = useApi()
const { servers, addServer, removeServer, setActiveServer, getActiveServerUrl, updateServerName, setToken } = useServerStore()
const { showToast, showConfirm } = useToastStore()

// ── Add Server form ───────────────────────────────────────
const newName = ref('')
const newUrl  = ref('')
const newUser = ref('')
const newPass = ref('')
const isAdding = ref(false)
const addError = ref(null)

const normalizeUrl = (raw) => {
  let url = raw.trim()
  if (url && !/^https?:\/\//i.test(url)) url = 'http://' + url
  return url.endsWith('/') ? url.slice(0, -1) : url
}

const handleAdd = async () => {
  addError.value = null

  if (!newName.value.trim() || !newUrl.value.trim() || !newUser.value.trim() || !newPass.value) {
    addError.value = 'All fields are required'
    return
  }

  const cleanUrl = normalizeUrl(newUrl.value)
  newUrl.value = cleanUrl
  isAdding.value = true

  try {
    // 1. Coba login ke server dulu
    const res = await fetch(`${cleanUrl}/api/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: newUser.value.trim(), password: newPass.value })
    })

    const data = await res.json()

    if (!res.ok) {
      addError.value = data.error || `Authentication failed (${res.status})`
      return
    }

    // 2. Login sukses → simpan server + token
    const id = Date.now().toString()
    addServer(newName.value.trim(), cleanUrl, id)
    setToken(id, data.token, data.username)

    // 3. Reset form
    newName.value = ''
    newUrl.value  = ''
    newUser.value = ''
    newPass.value = ''

    // 4. Langsung ke dashboard server baru
    setActiveServer(id)
    router.push(`/server/${id}/dashboard`)

  } catch (e) {
    addError.value = 'Cannot connect to server. Make sure backend is running.'
  } finally {
    isAdding.value = false
  }
}

// ── Server-specific actions ───────────────────────────────
const handleRenameServer = () => {
  const currentName = servers.value.find(s => s.id === route.params.id)?.name || ''
  const newNameStr = prompt('Masukkan nama baru untuk server ini:', currentName)
  if (newNameStr !== null && newNameStr.trim() !== '') {
    updateServerName(route.params.id, newNameStr.trim())
    showToast('Success', 'Server renamed successfully', 'success')
  }
}

const handleRemoveServer = () => {
  showConfirm(
    'Remove Server',
    'Apakah Anda yakin ingin menghapus server ini dari dashboard?',
    () => {
      removeServer(route.params.id)
      router.push('/')
      showToast('Removed', 'Server has been removed from dashboard', 'info')
    }
  )
}

const handleUpdate = () => {
  showConfirm(
    'Update Dashboard',
    'Pull changes and rebuild dashboard backend remotely?',
    async () => {
      try {
        const res = await apiFetch(`${getActiveServerUrl()}/api/system/update`, { method: 'POST' })
        const data = await res.json()
        if (res.ok) showToast('Success', data.message, 'success')
        else showToast('Error', data, 'error')
      } catch (e) {
        showToast('Failed', e.message, 'error')
      }
    }
  )
}

const handleReboot = () => {
  showConfirm(
    'DANGER: Reboot OS',
    'Reboot Host OS physically? This will disrupt all services.',
    async () => {
      try {
        const res = await apiFetch(`${getActiveServerUrl()}/api/system/reboot`, { method: 'POST' })
        const data = await res.json()
        if (res.ok) showToast('Rebooting', data.message, 'warning')
        else showToast('Error', data, 'error')
      } catch (e) {
        showToast('Failed', e.message, 'error')
      }
    }
  )
}

// ── System Reset handlers ────────────────────────────
const isResettingCloudflare = ref(false)
const isResettingUfw = ref(false)
const isResettingFail2ban = ref(false)

const handleResetCloudflare = () => {
  showConfirm(
    'Reset Cloudflare Configuration',
    'This will permanently remove the Cloudflare tunnel, config files, credentials, and auth certificate (cert.pem). You will need to re-authorize and create a new tunnel. Continue?',
    async () => {
      isResettingCloudflare.value = true
      try {
        const res = await apiFetch(`${getActiveServerUrl()}/api/cloudflare/reset`, { method: 'POST' })
        const data = await res.json()
        if (res.ok) showToast('Success', data.message, 'success')
        else showToast('Error', data.message || data, 'error')
      } catch (e) {
        showToast('Error', e.message, 'error')
      } finally {
        isResettingCloudflare.value = false
      }
    }
  )
}

const handleResetUfw = () => {
  showConfirm(
    'Reset UFW Firewall',
    'This will disable UFW and remove ALL firewall rules, restoring it to default state. Your server will be unprotected until you re-configure the firewall. Continue?',
    async () => {
      isResettingUfw.value = true
      try {
        const res = await apiFetch(`${getActiveServerUrl()}/api/firewall/reset`, { method: 'POST' })
        const data = await res.json()
        if (res.ok) showToast('Success', data.message, 'success')
        else showToast('Error', data.message || data, 'error')
      } catch (e) {
        showToast('Error', e.message, 'error')
      } finally {
        isResettingUfw.value = false
      }
    }
  )
}

const handleResetFail2ban = () => {
  showConfirm(
    'Reset Fail2Ban Configuration',
    'This will delete /etc/fail2ban/jail.local and restart Fail2Ban. All custom jails and ban rules will be removed. Continue?',
    async () => {
      isResettingFail2ban.value = true
      try {
        const res = await apiFetch(`${getActiveServerUrl()}/api/fail2ban/reset`, { method: 'POST' })
        const data = await res.json()
        if (res.ok) showToast('Success', data.message, 'success')
        else showToast('Error', data.message || data, 'error')
      } catch (e) {
        showToast('Error', e.message, 'error')
      } finally {
        isResettingFail2ban.value = false
      }
    }
  )
}
</script>

<template>
  <div class="space-y-6">

    <!-- ── Add Server (Global) ─────────────────────────── -->
    <section class="card" v-if="!route.params.id">
      <h2 class="card-title"><Settings class="w-5 h-5 text-brand-500" /> Global Configuration</h2>
      <p class="text-sm text-slate-500 mb-6">Add backend servers to monitor. Credentials are verified immediately via PAM.</p>

      <div class="p-5 border border-slate-200 bg-slate-50 rounded-lg space-y-4 dark:border-slate-700 dark:bg-slate-800">
        <h3 class="text-sm font-semibold dark:text-slate-200">Add New Backend Node</h3>

        <!-- Error banner -->
        <div v-if="addError" class="flex items-start gap-2 bg-red-50 border border-red-200 text-red-700 text-sm rounded-lg p-3 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300">
          <AlertCircle class="w-4 h-4 shrink-0 mt-0.5" />
          <span>{{ addError }}</span>
        </div>

        <!-- Server Name -->
        <div class="flex flex-col gap-1">
          <label class="text-sm font-medium text-slate-700 dark:text-slate-300">Server Name / Alias</label>
          <input v-model="newName" type="text" placeholder="e.g. VPS Singapore"
            class="input-field" :disabled="isAdding" />
        </div>

        <!-- IP / URL -->
        <div class="flex flex-col gap-1">
          <label class="text-sm font-medium text-slate-700 dark:text-slate-300">Backend IP / URL</label>
          <input v-model="newUrl" type="text" placeholder="100.127.55.109:8080"
            class="input-field" :disabled="isAdding" />
          <p class="text-xs text-slate-400">Cukup masukkan IP:Port — http:// akan ditambahkan otomatis</p>
        </div>

        <!-- Username + Password -->
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div class="flex flex-col gap-1">
            <label class="text-sm font-medium text-slate-700 dark:text-slate-300">Username</label>
            <div class="relative">
              <User class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none" />
              <input v-model="newUser" type="text" placeholder="e.g. root, ubuntu"
                class="input-field !pl-9" :disabled="isAdding" autocomplete="username" />
            </div>
          </div>
          <div class="flex flex-col gap-1">
            <label class="text-sm font-medium text-slate-700 dark:text-slate-300">Password</label>
            <div class="relative">
              <Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none" />
              <input v-model="newPass" type="password" placeholder="OS user password"
                class="input-field !pl-9" :disabled="isAdding" autocomplete="current-password"
                @keydown.enter="handleAdd" />
            </div>
          </div>
        </div>

        <!-- Submit -->
        <button @click="handleAdd" class="btn-primary w-full justify-center" :disabled="isAdding">
          <Loader2 v-if="isAdding" class="w-4 h-4 animate-spin" />
          <LogIn v-else class="w-4 h-4" />
          {{ isAdding ? 'Connecting & Authenticating...' : 'Add & Login' }}
        </button>
      </div>
    </section>

    <!-- ── Server Preferences (per-server) ────────────── -->
    <section class="card" v-if="route.params.id">
      <h2 class="card-title"><Settings class="w-5 h-5 text-brand-500" /> Server Preferences</h2>

      <div class="space-y-4 mb-6">
        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border rounded-lg gap-4 dark:border-slate-700 dark:bg-slate-800/50">
          <div>
            <div class="font-medium text-slate-800 dark:text-slate-200">Rename Server Alias</div>
            <div class="text-xs text-slate-500 dark:text-slate-400">Change how this server appears on the home screen</div>
          </div>
          <button @click="handleRenameServer" class="btn-outline whitespace-nowrap">
            <Edit2 class="w-4 h-4" /> Rename
          </button>
        </div>

        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border rounded-lg gap-4 dark:border-slate-700 dark:bg-slate-800/50">
          <div>
            <div class="font-medium text-slate-800 dark:text-slate-200">Remove Server</div>
            <div class="text-xs text-slate-500 dark:text-slate-400">Remove this server from your dashboard list</div>
          </div>
          <button @click="handleRemoveServer" class="btn-destructive whitespace-nowrap">
            <Trash2 class="w-4 h-4" /> Remove
          </button>
        </div>
      </div>
    </section>

    <!-- ── Danger Zone (per-server) ───────────────────── -->
    <section class="card border-red-200 dark:border-red-900/50" v-if="route.params.id">
      <h2 class="card-title text-red-600 dark:text-red-400"><Power class="w-5 h-5" /> Danger Zone</h2>
      <p class="text-sm text-slate-500 dark:text-slate-400 mb-6">Actions below affect this specific backend server OS ({{ getActiveServerUrl() }}).</p>

      <div class="space-y-4">
        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border rounded-lg gap-4 dark:border-slate-700 dark:bg-slate-800/50">
          <div>
            <div class="font-medium text-slate-800 dark:text-slate-200">Update Dashboard Backend</div>
            <div class="text-xs text-slate-500 dark:text-slate-400">Run git pull &amp; cargo build --release remotely</div>
          </div>
          <button @click="handleUpdate" class="btn-outline whitespace-nowrap">
            <RefreshCw class="w-4 h-4" /> Update Backend
          </button>
        </div>

        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border rounded-lg gap-4 dark:border-red-900/30 dark:bg-red-900/10">
          <div>
            <div class="font-medium text-red-800 dark:text-red-400">Reboot Host</div>
            <div class="text-xs text-red-600 dark:text-red-500">Reboot the physical operating system</div>
          </div>
          <button @click="handleReboot" class="btn-destructive whitespace-nowrap">
            <Power class="w-4 h-4" /> Reboot Server
          </button>
        </div>
      </div>
    </section>

    <!-- ── System Reset (per-server) ─────────────────── -->
    <section class="card border-orange-200 dark:border-orange-900/50" v-if="route.params.id">
      <h2 class="card-title text-orange-600 dark:text-orange-400"><RefreshCw class="w-5 h-5" /> System Reset</h2>
      <p class="text-sm text-slate-500 dark:text-slate-400 mb-6">
        Reset system components to their default state. These actions are irreversible and will remove all custom configuration.
      </p>

      <div class="space-y-3">
        <!-- Cloudflare Reset -->
        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border border-slate-200 dark:border-slate-700 rounded-lg gap-4 dark:bg-slate-800/50">
          <div class="flex items-start gap-3">
            <div class="w-8 h-8 rounded-lg bg-orange-100 dark:bg-orange-900/30 flex items-center justify-center shrink-0 mt-0.5">
              <Cloud class="w-4 h-4 text-orange-600 dark:text-orange-400" />
            </div>
            <div>
              <div class="font-medium text-slate-800 dark:text-slate-200">Reset Cloudflare</div>
              <div class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">Remove tunnel, config files, credentials, and auth certificate (cert.pem)</div>
            </div>
          </div>
          <button @click="handleResetCloudflare" class="btn-destructive whitespace-nowrap shrink-0" :disabled="isResettingCloudflare">
            <Loader2 v-if="isResettingCloudflare" class="w-4 h-4 animate-spin" />
            <Cloud v-else class="w-4 h-4" />
            {{ isResettingCloudflare ? 'Resetting...' : 'Reset Cloudflare' }}
          </button>
        </div>

        <!-- UFW Reset -->
        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border border-slate-200 dark:border-slate-700 rounded-lg gap-4 dark:bg-slate-800/50">
          <div class="flex items-start gap-3">
            <div class="w-8 h-8 rounded-lg bg-orange-100 dark:bg-orange-900/30 flex items-center justify-center shrink-0 mt-0.5">
              <Shield class="w-4 h-4 text-orange-600 dark:text-orange-400" />
            </div>
            <div>
              <div class="font-medium text-slate-800 dark:text-slate-200">Reset UFW Firewall</div>
              <div class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">Disable UFW and remove all firewall rules (restore to default)</div>
            </div>
          </div>
          <button @click="handleResetUfw" class="btn-destructive whitespace-nowrap shrink-0" :disabled="isResettingUfw">
            <Loader2 v-if="isResettingUfw" class="w-4 h-4 animate-spin" />
            <Shield v-else class="w-4 h-4" />
            {{ isResettingUfw ? 'Resetting...' : 'Reset UFW' }}
          </button>
        </div>

        <!-- Fail2Ban Reset -->
        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border border-slate-200 dark:border-slate-700 rounded-lg gap-4 dark:bg-slate-800/50">
          <div class="flex items-start gap-3">
            <div class="w-8 h-8 rounded-lg bg-orange-100 dark:bg-orange-900/30 flex items-center justify-center shrink-0 mt-0.5">
              <Ban class="w-4 h-4 text-orange-600 dark:text-orange-400" />
            </div>
            <div>
              <div class="font-medium text-slate-800 dark:text-slate-200">Reset Fail2Ban</div>
              <div class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">Delete jail.local and restart Fail2Ban (remove all custom jails and bans)</div>
            </div>
          </div>
          <button @click="handleResetFail2ban" class="btn-destructive whitespace-nowrap shrink-0" :disabled="isResettingFail2ban">
            <Loader2 v-if="isResettingFail2ban" class="w-4 h-4 animate-spin" />
            <Ban v-else class="w-4 h-4" />
            {{ isResettingFail2ban ? 'Resetting...' : 'Reset Fail2Ban' }}
          </button>
        </div>
      </div>
    </section>

  </div>
</template>
