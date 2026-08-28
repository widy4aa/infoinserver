<script setup>
import { ref, onMounted, nextTick } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { FileText, Loader2, RefreshCw, Filter, ArrowDownToLine } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast } = useToastStore()

const logs = ref('')
const parsedLogs = ref([])
const isLoading = ref(true)
const logType = ref('all') // all, auth, kernel
const logContainer = ref(null)

const parseLogLine = (line) => {
  if (!line.trim()) return null
  
  // journalctl -o short-iso format:
  // 2026-08-28T12:00:00+0000 hostname service_name[1234]: The actual message...
  
  const match = line.match(/^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[+-]\d{4})\s+([\w.-]+)\s+([^:]+):\s+(.*)$/)
  
  if (match) {
    return {
      timestamp: match[1],
      hostname: match[2],
      service: match[3],
      message: match[4]
    }
  }
  
  // Fallback for lines that don't match the exact format (like stack traces)
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
      
      // Parse raw text into structured objects
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
</script>

<template>
  <div class="space-y-6">
    <section class="card h-[85vh] flex flex-col p-0 overflow-hidden">
      
      <!-- Toolbar -->
      <div class="bg-slate-50 px-4 py-3 border-b border-slate-200 flex flex-col sm:flex-row sm:items-center justify-between shrink-0 gap-3">
        <div class="flex items-center gap-3">
          <div class="w-8 h-8 rounded-lg bg-blue-100 flex items-center justify-center border border-blue-200">
            <FileText class="w-4 h-4 text-blue-600" />
          </div>
          <div>
            <h2 class="text-sm font-bold text-slate-800 tracking-wide">System Journal</h2>
            <div class="text-[10px] text-slate-500 font-mono mt-0.5">/var/log/journal • {{ parsedLogs.length }} events</div>
          </div>
        </div>
        
        <div class="flex items-center gap-2 self-end sm:self-auto">
          <div class="flex items-center gap-2 bg-white px-3 py-1.5 rounded-lg border border-slate-200 shadow-sm">
            <Filter class="w-3.5 h-3.5 text-slate-400" />
            <select v-model="logType" @change="fetchSyslogs" class="bg-transparent font-medium text-xs text-slate-600 focus:outline-none cursor-pointer">
              <option value="all">All System Logs</option>
              <option value="auth">Auth / SSH Logs</option>
              <option value="kernel">Kernel Logs (dmesg)</option>
            </select>
          </div>
          
          <button @click="scrollToBottom" class="p-2 rounded-lg bg-white hover:bg-slate-50 text-slate-500 hover:text-slate-700 transition-colors border border-slate-200 shadow-sm" title="Scroll to bottom">
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
        class="flex-1 overflow-auto p-4 relative bg-white font-mono text-[11px] sm:text-[12px] leading-relaxed"
      >
        <div v-if="isLoading && parsedLogs.length === 0" class="absolute inset-0 flex flex-col items-center justify-center bg-white/80 z-10 backdrop-blur-sm">
          <Loader2 class="w-8 h-8 animate-spin text-brand-500 mb-3" />
          <span class="text-slate-500 font-mono text-xs">Querying journalctl...</span>
        </div>
        
        <div v-else-if="parsedLogs.length === 0" class="flex flex-col items-center justify-center h-full text-slate-400">
          <FileText class="w-12 h-12 mb-3 opacity-20" />
          <span>No logs found for this filter.</span>
        </div>

        <div v-else class="space-y-1 pb-4">
          <div v-for="(log, idx) in parsedLogs" :key="idx" class="flex flex-col sm:flex-row hover:bg-slate-50 py-1 px-2 rounded-md transition-colors border border-transparent hover:border-slate-100 group">
            
            <template v-if="log.timestamp">
              <!-- Structured Log Line -->
              <div class="flex items-start sm:w-auto shrink-0 mr-3 text-slate-400 group-hover:text-slate-500 transition-colors">
                <span class="min-w-[170px] inline-block">{{ log.timestamp.replace('T', ' ').split('+')[0] }}</span>
                <span class="text-emerald-600 min-w-[100px] inline-block ml-2 font-medium">{{ log.hostname }}</span>
              </div>
              
              <div class="flex-1 min-w-0 mt-1 sm:mt-0 flex flex-col sm:flex-row items-start">
                <span class="text-blue-600 font-semibold sm:w-48 shrink-0 mr-3 truncate" :title="log.service">
                  {{ log.service }}
                </span>
                
                <span class="text-slate-700 break-words flex-1" 
                      :class="{'text-red-600 font-bold bg-red-50 px-1 rounded -ml-1': log.message.toLowerCase().includes('error') || log.message.toLowerCase().includes('failed') || log.message.toLowerCase().includes('fatal'),
                               'text-amber-600 font-medium': log.message.toLowerCase().includes('warn')}">
                  {{ log.message }}
                </span>
              </div>
            </template>
            
            <template v-else>
              <!-- Unstructured Line (Fallback) -->
              <div class="text-slate-500 break-words pl-0 sm:pl-[280px] w-full italic">
                {{ log.message }}
              </div>
            </template>
            
          </div>
        </div>
      </div>
      
    </section>
  </div>
</template>

<style scoped>
/* Custom Scrollbar for light theme */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}
::-webkit-scrollbar-track {
  background: #f8fafc; 
}
::-webkit-scrollbar-thumb {
  background: #cbd5e1; 
  border-radius: 4px;
}
::-webkit-scrollbar-thumb:hover {
  background: #94a3b8; 
}
</style>
