<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import { FileText, Loader2, Pause, Play, RefreshCw, Activity, Terminal } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast } = useToastStore()
const { isDark } = useThemeStore()

// ── TABS STATE ──
const activeTab = ref('journal') // 'journal', 'activity', 'bash'

// ── SYSTEM JOURNAL STATE ──
const rawLogs = ref([])
const isAutoRefresh = ref(true)
const isLoading = ref(true)
let refreshInterval = null
const logContainer = ref(null)

const parsedLogs = computed(() => {
  return rawLogs.value.map(line => {
    const parts = line.split(' ')
    if (parts.length >= 5) {
      return {
        timestamp: `${parts[0]} ${parts[1]}`,
        hostname: parts[2],
        service: parts[3].replace(':', ''),
        message: parts.slice(4).join(' ')
      }
    }
    return { message: line, isFallback: true }
  })
})

const fetchSyslogs = async () => {
  if (activeTab.value !== 'journal') return
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/syslogs?filter=all`)
    if (res.ok) {
      const data = await res.json()
      // Split the single string block into an array of lines, remove empty lines
      rawLogs.value = (data.logs || '').split('\n').filter(l => l.trim() !== '')
      scrollToBottom(logContainer.value)
    } else {
      const err = await res.text()
      showToast('Error', err, 'error')
    }
  } catch (e) {
    console.error(e)
  } finally {
    isLoading.value = false
  }
}

// ── DASHBOARD ACTIVITY STATE ──
const activityLogs = ref([])
const isLoadingActivity = ref(false)

const fetchActivityLogs = async () => {
  if (activeTab.value !== 'activity') return
  isLoadingActivity.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/logs/activity`)
    if (res.ok) {
      activityLogs.value = await res.json()
    }
  } catch (e) {
    console.error(e)
  } finally {
    isLoadingActivity.value = false
  }
}

// ── BASH HISTORY STATE ──
const bashHistory = ref([])
const isLoadingBash = ref(false)
const bashContainer = ref(null)

const fetchBashHistory = async () => {
  if (activeTab.value !== 'bash') return
  isLoadingBash.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/logs/bash_history`)
    if (res.ok) {
      bashHistory.value = await res.json()
      scrollToBottom(bashContainer.value)
    }
  } catch (e) {
    console.error(e)
  } finally {
    isLoadingBash.value = false
  }
}

// ── UTILS ──
const toggleAutoRefresh = () => {
  isAutoRefresh.value = !isAutoRefresh.value
  if (isAutoRefresh.value) {
    fetchSyslogs()
    refreshInterval = setInterval(fetchSyslogs, 3000)
  } else {
    clearInterval(refreshInterval)
  }
}

const scrollToBottom = (el) => {
  if (!el) return
  setTimeout(() => {
    const isScrolledToBottom = el.scrollHeight - el.clientHeight <= el.scrollTop + 100
    if (isScrolledToBottom || rawLogs.value.length === 0 || bashHistory.value.length === 0) {
      el.scrollTop = el.scrollHeight
    }
  }, 50)
}

const changeTab = (tab) => {
  activeTab.value = tab
  if (tab === 'journal') fetchSyslogs()
  else if (tab === 'activity') fetchActivityLogs()
  else if (tab === 'bash') fetchBashHistory()
}

onMounted(() => {
  fetchSyslogs()
  refreshInterval = setInterval(() => {
    if (isAutoRefresh.value && activeTab.value === 'journal') {
      fetchSyslogs()
    }
  }, 3000)
})

onUnmounted(() => {
  if (refreshInterval) clearInterval(refreshInterval)
})

const getLevelColor = (level) => {
  switch(level?.toUpperCase()) {
    case 'CRITICAL': return 'bg-red-500/20 text-red-500 border border-red-500/50'
    case 'WARNING': return 'bg-amber-500/20 text-amber-500 border border-amber-500/50'
    case 'INFO': return 'bg-blue-500/20 text-blue-500 border border-blue-500/50'
    default: return 'bg-slate-500/20 text-slate-500 border border-slate-500/50'
  }
}
</script>

<template>
  <div class="space-y-4">
    <!-- Tabs Header -->
    <div class="flex items-center gap-2 border-b" :class="isDark ? 'border-slate-800' : 'border-slate-200'">
      <button @click="changeTab('journal')" class="px-4 py-2 text-sm font-semibold transition-colors border-b-2"
        :class="activeTab === 'journal' ? 'border-blue-500 text-blue-500' : 'border-transparent text-slate-500 hover:text-slate-700'">
        <div class="flex items-center gap-2"><FileText class="w-4 h-4"/> System Journal</div>
      </button>
      <button @click="changeTab('activity')" class="px-4 py-2 text-sm font-semibold transition-colors border-b-2"
        :class="activeTab === 'activity' ? 'border-blue-500 text-blue-500' : 'border-transparent text-slate-500 hover:text-slate-700'">
        <div class="flex items-center gap-2"><Activity class="w-4 h-4"/> Dashboard Activity</div>
      </button>
      <button @click="changeTab('bash')" class="px-4 py-2 text-sm font-semibold transition-colors border-b-2"
        :class="activeTab === 'bash' ? 'border-blue-500 text-blue-500' : 'border-transparent text-slate-500 hover:text-slate-700'">
        <div class="flex items-center gap-2"><Terminal class="w-4 h-4"/> Bash History</div>
      </button>
    </div>

    <!-- TAB 1: JOURNAL -->
    <section v-if="activeTab === 'journal'" class="card h-[80vh] flex flex-col p-0 overflow-hidden">
      <!-- Toolbar -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between shrink-0 gap-3 px-4 py-3 border-b"
           :class="isDark ? 'bg-slate-800/50 border-slate-800' : 'bg-slate-50 border-slate-200'">
        <div class="flex items-center gap-3">
          <div>
            <h2 class="text-sm font-bold tracking-wide" :class="isDark ? 'text-slate-100' : 'text-slate-800'">System Journal</h2>
            <div class="text-[10px] font-mono mt-0.5" :class="isDark ? 'text-slate-500' : 'text-slate-500'">/var/log/journal</div>
          </div>
        </div>
        
        <div class="flex items-center gap-2 self-end sm:self-auto">
          <div class="flex items-center gap-2 px-3 py-1.5 rounded-lg border shadow-sm"
               :class="isDark ? 'bg-slate-800 border-slate-700 text-slate-300' : 'bg-white border-slate-200 text-slate-600'">
            <div class="relative flex h-2 w-2">
              <span v-if="isAutoRefresh" class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
              <span class="relative inline-flex rounded-full h-2 w-2" :class="isAutoRefresh ? 'bg-emerald-500' : 'bg-slate-400'"></span>
            </div>
            <span class="text-xs font-medium">{{ isAutoRefresh ? 'Live' : 'Paused' }}</span>
          </div>

          <button @click="toggleAutoRefresh" class="btn-outline h-8 px-3 text-xs" :title="isAutoRefresh ? 'Pause auto-refresh' : 'Resume auto-refresh'">
            <Pause v-if="isAutoRefresh" class="w-3.5 h-3.5" />
            <Play v-else class="w-3.5 h-3.5" />
          </button>
          
          <button @click="fetchSyslogs" class="btn-primary h-8 px-3 text-xs shadow-sm" :disabled="isLoading">
            <Loader2 v-if="isLoading" class="w-3.5 h-3.5 animate-spin" />
            <RefreshCw v-else class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
      
      <!-- Log Content -->
      <div class="flex-1 overflow-y-auto p-4 font-mono text-[11px] leading-relaxed scroll-smooth"
           :class="isDark ? 'bg-[#0f111a] text-slate-300' : 'bg-slate-900 text-slate-300'"
           ref="logContainer">
        
        <div v-if="isLoading && rawLogs.length === 0" class="flex flex-col items-center justify-center h-full text-slate-500 space-y-3">
          <Loader2 class="w-6 h-6 animate-spin text-blue-500" />
          <p>Loading journalctl logs...</p>
        </div>

        <div v-else class="flex flex-col">
          <div v-for="(log, idx) in parsedLogs" :key="idx" 
               class="flex flex-col sm:flex-row py-0.5 hover:bg-white/5 transition-colors group rounded px-1">
            <template v-if="log.timestamp">
              <div class="flex items-start sm:w-auto shrink-0 mr-3 text-slate-400">
                <span class="min-w-[170px] inline-block">{{ log.timestamp.replace('T', ' ').split('+')[0] }}</span>
                <span class="min-w-[100px] inline-block ml-2 font-medium text-emerald-400">{{ log.hostname }}</span>
              </div>
              <div class="flex-1 min-w-0 mt-1 sm:mt-0 flex flex-col sm:flex-row items-start">
                <span class="font-semibold sm:w-48 shrink-0 mr-3 truncate text-blue-400" :title="log.service">
                  {{ log.service }}
                </span>
                <span class="break-words flex-1"
                      :class="(log.message.toLowerCase().includes('error') || log.message.toLowerCase().includes('failed') || log.message.toLowerCase().includes('fatal')
                            ? 'text-red-400 font-bold bg-red-900/30 px-1 rounded -ml-1'
                            : log.message.toLowerCase().includes('warn')
                                ? 'text-amber-400 font-medium'
                                : 'text-slate-200')">
                  {{ log.message }}
                </span>
              </div>
            </template>
            <template v-else>
              <div class="break-words pl-0 sm:pl-[280px] w-full italic text-slate-500">
                {{ log.message }}
              </div>
            </template>
          </div>
        </div>
      </div>
    </section>

    <!-- TAB 2: ACTIVITY LOGS -->
    <section v-if="activeTab === 'activity'" class="card p-0 overflow-hidden">
      <div class="flex items-center justify-between px-4 py-3 border-b" :class="isDark ? 'border-slate-800 bg-slate-800/50' : 'border-slate-200 bg-slate-50'">
        <h2 class="text-sm font-bold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">Dashboard Action Audit</h2>
        <button @click="fetchActivityLogs" class="btn-outline text-xs h-8 px-3" :disabled="isLoadingActivity">
          <RefreshCw :class="{'animate-spin': isLoadingActivity}" class="w-3.5 h-3.5" />
        </button>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead class="border-b" :class="isDark ? 'bg-slate-800/80 border-slate-700' : 'bg-slate-100 border-slate-200'">
            <tr>
              <th class="table-th text-xs">Timestamp</th>
              <th class="table-th text-xs">Level</th>
              <th class="table-th text-xs">Action</th>
              <th class="table-th text-xs">Details</th>
            </tr>
          </thead>
          <tbody class="divide-y" :class="isDark ? 'divide-slate-800/50' : 'divide-slate-100'">
            <tr v-for="log in activityLogs" :key="log.id" class="text-sm" :class="isDark ? 'hover:bg-slate-800/30' : 'hover:bg-slate-50'">
              <td class="table-td whitespace-nowrap text-slate-500 text-xs">{{ new Date(log.timestamp).toLocaleString() }}</td>
              <td class="table-td whitespace-nowrap">
                <span class="px-2 py-0.5 rounded text-[10px] font-bold tracking-wider uppercase" :class="getLevelColor(log.level)">
                  {{ log.level }}
                </span>
              </td>
              <td class="table-td font-semibold" :class="isDark ? 'text-slate-300' : 'text-slate-700'">{{ log.action }}</td>
              <td class="table-td text-slate-500">{{ log.detail || '-' }}</td>
            </tr>
            <tr v-if="activityLogs.length === 0 && !isLoadingActivity">
              <td colspan="4" class="text-center p-8 text-slate-500 italic text-sm">No dashboard activity recorded yet.</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- TAB 3: BASH HISTORY -->
    <section v-if="activeTab === 'bash'" class="card h-[80vh] flex flex-col p-0 overflow-hidden">
      <div class="flex items-center justify-between px-4 py-3 border-b shrink-0" :class="isDark ? 'border-slate-800 bg-slate-800/50' : 'border-slate-200 bg-slate-50'">
        <div>
          <h2 class="text-sm font-bold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">Terminal Commands History</h2>
          <div class="text-[10px] text-slate-500 mt-0.5 font-mono">~/.bash_history</div>
        </div>
        <button @click="fetchBashHistory" class="btn-outline text-xs h-8 px-3" :disabled="isLoadingBash">
          <RefreshCw :class="{'animate-spin': isLoadingBash}" class="w-3.5 h-3.5" />
        </button>
      </div>
      <div class="flex-1 overflow-y-auto p-4 font-mono text-xs leading-relaxed bg-[#0f111a] text-green-400" ref="bashContainer">
        <div v-if="isLoadingBash && bashHistory.length === 0" class="flex flex-col items-center justify-center h-full text-slate-500">
          <Loader2 class="w-6 h-6 animate-spin mb-2" /> Loading history...
        </div>
        <div v-else>
          <div v-for="(cmd, i) in bashHistory" :key="i" class="py-0.5 hover:bg-white/5 px-1 rounded">
            <span class="text-blue-400 opacity-70 select-none mr-2">{{ i+1 }}</span>
            <span class="text-slate-400 mr-2 select-none" v-if="cmd.startsWith('[')">{{ cmd.split(']')[0] + ']' }}</span>
            <span class="text-green-300 font-semibold">{{ cmd.includes(']') ? cmd.substring(cmd.indexOf(']')+1).trim() : cmd }}</span>
          </div>
          <div v-if="bashHistory.length === 0 && !isLoadingBash" class="text-slate-500 italic">No bash history found.</div>
        </div>
      </div>
    </section>

  </div>
</template>