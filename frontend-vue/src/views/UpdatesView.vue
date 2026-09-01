<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import { Download, RefreshCw, Terminal, CheckCircle2, AlertTriangle, Box } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl, getToken, activeServerId } = useServerStore()
const { showToast } = useToastStore()
const { isDark } = useThemeStore()

const updateInfo = ref(null)
const isChecking = ref(true)

const showUpgradeModal = ref(false)
const upgradeLogs = ref([])
let ws = null
const logsContainer = ref(null)

const checkUpdates = async () => {
  isChecking.value = true
  updateInfo.value = null
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/system/os_updates`)
    if (res.ok) {
      updateInfo.value = await res.json()
    } else {
      showToast('Error', await res.text(), 'error')
    }
  } catch (e) {
    showToast('Error', 'Failed to check OS updates', 'error')
  } finally {
    isChecking.value = false
  }
}

const startUpgrade = () => {
  showUpgradeModal.value = true
  upgradeLogs.value = []
  
  const token = getToken(activeServerId.value)
  const wsUrl = getActiveServerUrl().replace(/^http/, 'ws') + '/api/system/os_updates/ws'
               + (token ? '?token=' + encodeURIComponent(token) : '')
  ws = new WebSocket(wsUrl)

  ws.onopen = () => {
    upgradeLogs.value.push("--- Starting System Upgrade ---")
  }

  ws.onmessage = (event) => {
    upgradeLogs.value.push(event.data)
    setTimeout(() => {
      if (logsContainer.value) {
        logsContainer.value.scrollTop = logsContainer.value.scrollHeight
      }
    }, 10)
  }

  ws.onclose = () => {
    upgradeLogs.value.push("--- Connection Closed ---")
    checkUpdates() // Re-check after upgrade
  }
}

const closeUpgradeModal = () => {
  if (ws) ws.close()
  showUpgradeModal.value = false
}

onMounted(checkUpdates)

onUnmounted(() => {
  if (ws) ws.close()
})
</script>

<template>
  <div class="space-y-6">
    <section class="card">
      <div class="flex items-center justify-between mb-6">
        <div>
          <h2 class="card-title mb-1"><Download class="w-5 h-5 text-brand-500" /> OS Package Manager</h2>
          <p class="text-xs text-slate-500">Check and install security updates for your Linux distribution.</p>
        </div>
        <button @click="checkUpdates" class="btn-outline" :disabled="isChecking">
          <RefreshCw :class="{'animate-spin': isChecking}" class="w-4 h-4" />
          {{ isChecking ? 'Checking...' : 'Check Updates' }}
        </button>
      </div>

      <div v-if="isChecking" class="p-12 text-center text-slate-500 flex flex-col items-center gap-3">
        <RefreshCw class="w-8 h-8 animate-spin text-brand-400" />
        <p>Syncing package repositories... This might take a minute.</p>
      </div>

      <div v-else-if="updateInfo">
        <div class="flex flex-col md:flex-row gap-6 mb-6">
          <div class="flex-1 p-6 rounded-xl border flex flex-col items-center justify-center text-center gap-2"
               :class="isDark ? 'bg-slate-800/50 border-slate-700' : 'bg-slate-50 border-slate-200'">
            <span class="text-xs font-bold uppercase tracking-wider text-slate-500">Package Manager</span>
            <span class="text-2xl font-mono text-brand-500">{{ updateInfo.manager }}</span>
          </div>
          
          <div class="flex-1 p-6 rounded-xl border flex flex-col items-center justify-center text-center gap-2"
               :class="updateInfo.updatable_count > 0 
                  ? (isDark ? 'bg-amber-900/20 border-amber-500/50' : 'bg-amber-50 border-amber-200')
                  : (isDark ? 'bg-green-900/20 border-green-500/50' : 'bg-green-50 border-green-200')">
            <span class="text-xs font-bold uppercase tracking-wider" :class="updateInfo.updatable_count > 0 ? 'text-amber-500' : 'text-green-500'">Updatable Packages</span>
            <span class="text-4xl font-bold" :class="updateInfo.updatable_count > 0 ? 'text-amber-600 dark:text-amber-400' : 'text-green-600 dark:text-green-400'">{{ updateInfo.updatable_count }}</span>
          </div>
        </div>

        <div v-if="updateInfo.updatable_count > 0" class="space-y-4">
          <div class="flex items-center justify-between">
            <h3 class="font-bold text-sm">Available Updates</h3>
            <button @click="startUpgrade" class="btn-primary bg-amber-500 hover:bg-amber-600 text-white border-none">
              <Download class="w-4 h-4" /> Upgrade All Packages
            </button>
          </div>
          <div class="bg-[#0f111a] text-slate-300 font-mono text-[11px] p-4 rounded-lg overflow-y-auto max-h-96 shadow-inner">
            <div v-for="(pkg, i) in updateInfo.details" :key="i" class="py-0.5 hover:bg-white/5 px-1">
              {{ pkg }}
            </div>
          </div>
        </div>
        <div v-else class="text-center p-8 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-xl">
          <CheckCircle2 class="w-12 h-12 text-green-500 mx-auto mb-3" />
          <h3 class="font-bold text-green-700 dark:text-green-400 text-lg">System is Up to Date</h3>
          <p class="text-sm text-green-600 dark:text-green-500 mt-1">No pending updates found for your distribution.</p>
        </div>
      </div>
    </section>

    <!-- Modal Terminal -->
    <Teleport to="body">
      <div v-if="showUpgradeModal" class="fixed inset-0 z-[100] backdrop-blur-sm flex items-center justify-center p-4 bg-slate-900/80">
        <div class="rounded-xl shadow-2xl w-full max-w-4xl overflow-hidden flex flex-col h-[80vh]" :class="isDark ? 'bg-slate-900 border border-slate-700' : 'bg-slate-900'">
          <div class="p-3 border-b border-slate-700 flex justify-between items-center bg-slate-950">
            <h3 class="font-bold text-slate-200 flex items-center gap-2"><Terminal class="w-4 h-4 text-brand-500"/> OS Upgrade Process</h3>
            <button @click="closeUpgradeModal" class="text-slate-400 hover:text-white px-2 py-1 text-xs border border-slate-700 rounded">Close</button>
          </div>
          <div ref="logsContainer" class="flex-1 p-4 font-mono text-[11px] overflow-y-auto text-slate-300 leading-relaxed scroll-smooth bg-black">
            <div v-for="(line, i) in upgradeLogs" :key="i" :class="{'text-green-400': line.includes('Setting up') || line.includes('Finished')}">{{ line }}</div>
            <div v-if="upgradeLogs.length === 0" class="text-slate-500">Connecting to upgrade stream...</div>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>