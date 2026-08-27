<script setup>
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { X, Loader2, Terminal as TerminalIcon } from 'lucide-vue-next'
import { useServerStore } from '../stores/serverStore'
import '@xterm/xterm/css/xterm.css'

const emit = defineEmits(['close'])
const { getActiveServerUrl } = useServerStore()

const terminalContainer = ref(null)
const isLoading = ref(true)
const connectionError = ref(null)

let term = null
let fitAddon = null
let ws = null

const initTerminal = async () => {
  if (!terminalContainer.value) return

  term = new Terminal({
    cursorBlink: true,
    theme: {
      background: '#0f172a',
      foreground: '#f8fafc'
    },
    fontFamily: 'Menlo, Monaco, "Courier New", monospace'
  })

  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)
  term.open(terminalContainer.value)
  
  // Wait a tick for DOM to size properly
  await nextTick()
  fitAddon.fit()

  term.writeln('Connecting to backend server...')

  try {
    const baseUrl = getActiveServerUrl()
    // Convert http:// to ws://
    const wsUrl = baseUrl.replace(/^http/, 'ws') + '/api/terminal/ws'
    
    ws = new WebSocket(wsUrl)
    ws.binaryType = "arraybuffer"

    ws.onopen = () => {
      isLoading.value = false
      term.clear()
      term.focus()
    }

    ws.onmessage = (evt) => {
      if (evt.data instanceof ArrayBuffer) {
        term.write(new Uint8Array(evt.data))
      } else {
        term.write(evt.data)
      }
    }

    ws.onclose = () => {
      term.writeln('\n\r\x1b[31mConnection Closed.\x1b[0m')
    }

    ws.onerror = (err) => {
      isLoading.value = false
      connectionError.value = "WebSocket connection failed. Ensure backend is running and CORS is enabled."
      term.writeln('\n\r\x1b[31mConnection Error.\x1b[0m')
    }

    term.onData((data) => {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(data)
      }
    })

  } catch (err) {
    isLoading.value = false
    connectionError.value = err.message
  }
}

const handleResize = () => {
  if (fitAddon) {
    fitAddon.fit()
  }
}

onMounted(() => {
  window.addEventListener('resize', handleResize)
  // Beri sedikit delay untuk memastikan animasi modal selesai dan tinggi div terhitung dengan benar
  setTimeout(initTerminal, 300)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  if (ws) {
    ws.close()
    ws = null
  }
  if (term) {
    term.dispose()
    term = null
  }
})
</script>

<template>
  <div class="fixed inset-0 bg-slate-900/80 z-[60] flex items-center justify-center p-4 backdrop-blur-sm">
    <div class="bg-slate-900 rounded-lg w-full max-w-5xl h-[80vh] flex flex-col overflow-hidden shadow-2xl border border-slate-700">
      
      <!-- Header -->
      <div class="px-4 py-3 flex justify-between items-center bg-slate-800 border-b border-slate-700">
        <div class="flex items-center gap-2 text-slate-300 text-sm font-medium">
          <TerminalIcon class="w-4 h-4 text-brand-400" /> Root Terminal
        </div>
        <button @click="$emit('close')" class="text-slate-400 hover:text-white transition-colors bg-slate-700 hover:bg-slate-600 rounded p-1">
          <X class="w-4 h-4" />
        </button>
      </div>
      
      <!-- Body -->
      <div class="relative flex-1 w-full p-2 bg-slate-900 overflow-hidden">
        
        <!-- Loading State -->
        <div v-if="isLoading && !connectionError" class="absolute inset-0 flex flex-col items-center justify-center text-slate-400 z-10 bg-slate-900">
          <Loader2 class="w-8 h-8 animate-spin mb-3 text-brand-500" />
          <p class="text-sm">Initiating PTY Session...</p>
        </div>

        <!-- Error State -->
        <div v-if="connectionError" class="absolute inset-0 flex flex-col items-center justify-center z-10 bg-slate-900/90 p-6 text-center">
          <div class="bg-red-500/10 border border-red-500/50 rounded-lg p-4 max-w-md">
            <h3 class="text-red-400 font-bold mb-2">Connection Failed</h3>
            <p class="text-red-300 text-sm">{{ connectionError }}</p>
            <button @click="$emit('close')" class="mt-4 px-4 py-2 bg-slate-800 text-white text-sm rounded hover:bg-slate-700">Close Terminal</button>
          </div>
        </div>

        <!-- XTerm Container -->
        <div ref="terminalContainer" class="w-full h-full"></div>
      </div>
    </div>
  </div>
</template>

<style>
/* Reset xterm scrollbar to look modern */
.xterm-viewport::-webkit-scrollbar {
  width: 10px;
}
.xterm-viewport::-webkit-scrollbar-track {
  background: #0f172a;
}
.xterm-viewport::-webkit-scrollbar-thumb {
  background: #334155;
  border-radius: 5px;
}
.xterm-viewport::-webkit-scrollbar-thumb:hover {
  background: #475569;
}
</style>
