<script setup>
import { ref, onMounted, nextTick } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import { FileText, Loader2, RefreshCw, Filter, ArrowDownToLine } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast } = useToastStore()
const { isDark } = useThemeStore()

const logs = ref('')
const parsedLogs = ref([])
const isLoading = ref(true)
const logType = ref('all')
const logContainer = ref(null)

const parseLogLine = (line) => {
  if (!line.trim()) return null
  
  const match = line.match(/^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[+-]\d{4})\s+([\w.-]+)\s+([^:]+):\s+(.*)$/)
  
  if (match) {
    return {
      timestamp: match[1],
      hostname: match[2],
      service: match[3],
      message: match[4]
    }
  }
  
  return {
    timestamp: '',
    hostname: '',
    service: '',
    message: line
  }
}

const scrollToBottom = async () => {
  await nextTick()
  if (logContainer.value) {
    logContainer.value.scrollTop = logContainer.value.scrollHeight
  }
}

const fetchSyslogs = async () => {
  isLoading.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/syslogs?filter=${logType.value}`)
    if (res.ok) {
      const data = await res.json()
      logs.value = data.logs
      
      parsedLogs.value = data.logs
        .split('\n')
        .map(parseLogLine)
        .filter(Boolean)
        
      scrollToBottom()
    } else {
      const err = await res.json()
      showToast("Error", err.error || "Failed to fetch syslogs", "error")
    }
  } catch (e) {
    showToast("Error", "Network error", "error")
  } finally {
    isLoading.value = false
  }
}

onMounted(fetchSyslogs)

const getScrollbarStyles = () => {
  if (isDark.value) {
    return `
      ::-webkit-scrollbar { width: 8px; height: 8px; }
      ::-webkit-scrollbar-track { background: #0f172a; }
      ::-webkit-scrollbar-thumb { background: #334155; border-radius: 4px; }
      ::-webkit-scrollbar-thumb:hover { background: #475569; }
    `
  }
  return `
    ::-webkit-scrollbar { width: 8px; height: 8px; }
    ::-webkit-scrollbar-track { background: #f8fafc; }
    ::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 4px; }
    ::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
  `
}
</script>

<template>
  <div class="space-y-6">
    <section class="card h-[85vh] flex flex-col p-0 overflow-hidden">
      
      <!-- Toolbar -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between shrink-0 gap-3 px-4 py-3 border-b"
           :class="isDark ? 'bg-slate-800/50 border-slate-800' : 'bg-slate-50 border-slate-200'">
        <div class="flex items-center gap-3">
          <div class="w-8 h-8 rounded-lg flex items-center justify-center border"
               :class="isDark ? 'bg-slate-800 border-slate-700' : 'bg-blue-100 border-blue-200'">
            <FileText class="w-4 h-4" :class="isDark ? 'text-blue-400' : 'text-blue-600'" />
          </div>
          <div>
            <h2 class="text-sm font-bold tracking-wide" :class="isDark ? 'text-slate-100' : 'text-slate-800'">System Journal</h2>
            <div class="text-[10px] font-mono mt-0.5" :class="isDark ? 'text-slate-500' : 'text-slate-500'">/var/log/journal • {{ parsedLogs.length }} events</div>
          </div>
        </div>
        
        <div class="flex items-center gap-2 self-end sm:self-auto">
          <div class="flex items-center gap-2 px-3 py-1.5 rounded-lg border shadow-sm"
               :class="isDark ? 'bg-slate-800 border-slate-700 text-slate-300' : 'bg-white border-slate-200 text-slate-600'">
            <Filter class="w-3.5 h-3.5" :class="isDark ? 'text-slate-500' : 'text-slate-400'" />
            <select v-model="logType" @change="fetchSyslogs" class="bg-transparent font-medium text-xs focus:outline-none cursor-pointer"
                    :class="isDark ? 'text-slate-300' : 'text-slate-600'">
              <option value="all">All System Logs</option>
              <option value="auth">Auth / SSH Logs</option>
              <option value="kernel">Kernel Logs (dmesg)</option>
            </select>
          </div>
          
          <button @click="scrollToBottom" class="p-2 rounded-lg transition-colors border shadow-sm"
                  :class="isDark ? 'bg-slate-800 border-slate-700 text-slate-400 hover:bg-slate-700 hover:text-slate-200' : 'bg-white border-slate-200 text-slate-500 hover:bg-slate-50 hover:text-slate-700'"
                  title="Scroll to bottom">
            <ArrowDownToLine class="w-4 h-4" />
          </button>
          
          <button @click="fetchSyslogs" class="btn-primary !px-3 !py-2" title="Refresh">
            <RefreshCw class="w-4 h-4" :class="{'animate-spin': isLoading}" />
          </button>
        </div>
      </div>

      <!-- Log Viewer -->
      <div 
        ref="logContainer"
        class="flex-1 overflow-auto p-4 relative font-mono text-[11px] sm:text-[12px] leading-relaxed"
        :class="isDark ? 'bg-slate-950 text-slate-100' : 'bg-white text-slate-700'"
        :style="getScrollbarStyles()"
      >
        <div v-if="isLoading && parsedLogs.length === 0" class="absolute inset-0 flex flex-col items-center justify-center z-10 backdrop-blur-sm"
             :class="isDark ? 'bg-slate-950/80' : 'bg-white/80'">
          <Loader2 class="w-8 h-8 animate-spin text-brand-500 mb-3" />
          <span class="font-mono text-xs" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Querying journalctl...</span>
        </div>
        
        <div v-else-if="parsedLogs.length === 0" class="flex flex-col items-center justify-center h-full"
             :class="isDark ? 'text-slate-500' : 'text-slate-400'">
          <FileText class="w-12 h-12 mb-3 opacity-20" />
          <span>No logs found for this filter.</span>
        </div>

        <div v-else class="space-y-1 pb-4">
          <div v-for="(log, idx) in parsedLogs" :key="idx" class="flex flex-col sm:flex-row hover:py-1 px-2 rounded-md transition-colors border border-transparent group"
               :class="isDark ? 'hover:bg-slate-800/50 hover:border-slate-700' : 'hover:bg-slate-50 hover:border-slate-100'">
            
            <template v-if="log.timestamp">
              <!-- Structured Log Line -->
              <div class="flex items-start sm:w-auto shrink-0 mr-3"
                   :class="isDark ? 'text-slate-400 group-hover:text-slate-300' : 'text-slate-400 group-hover:text-slate-500'">
                <span class="min-w-[170px] inline-block">{{ log.timestamp.replace('T', ' ').split('+')[0] }}</span>
                <span class="min-w-[100px] inline-block ml-2 font-medium text-emerald-400">{{ log.hostname }}</span>
              </div>
              
              <div class="flex-1 min-w-0 mt-1 sm:mt-0 flex flex-col sm:flex-row items-start">
                <span class="font-semibold sm:w-48 shrink-0 mr-3 truncate text-blue-400" :title="log.service">
                  {{ log.service }}
                </span>
                
                <span class="break-words flex-1"
                      :class="isDark
                        ? (log.message.toLowerCase().includes('error') || log.message.toLowerCase().includes('failed') || log.message.toLowerCase().includes('fatal')
                            ? 'text-red-400 font-bold bg-red-900/30 px-1 rounded -ml-1'
                            : log.message.toLowerCase().includes('warn')
                                ? 'text-amber-400 font-medium'
                                : 'text-slate-200')
                        : (log.message.toLowerCase().includes('error') || log.message.toLowerCase().includes('failed') || log.message.toLowerCase().includes('fatal')
                            ? 'text-red-600 font-bold bg-red-50 px-1 rounded -ml-1'
                            : log.message.toLowerCase().includes('warn')
                                ? 'text-amber-600 font-medium'
                                : 'text-slate-700')">
                  {{ log.message }}
                </span>
              </div>
            </template>
            
            <template v-else>
              <!-- Unstructured Line (Fallback) -->
              <div class="break-words pl-0 sm:pl-[280px] w-full italic"
                   :class="isDark ? 'text-slate-500' : 'text-slate-500'">
                {{ log.message }}
              </div>
            </template>
            
          </div>
        </div>
      </div>
      
    </section>
  </div>
</template>