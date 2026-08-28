<script setup>
import { ref, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { X, Loader2, Terminal as TerminalIcon, Plus } from 'lucide-vue-next'
import { useServerStore } from '../stores/serverStore'
import '@xterm/xterm/css/xterm.css'

const props = defineProps({ visible: Boolean })
const emit = defineEmits(['close'])
const { getActiveServerUrl, getToken, activeServerId } = useServerStore()

// ── Tab state ─────────────────────────────────────────────
let tabIdCounter = 0

const createTab = () => ({
  id: ++tabIdCounter,
  label: `Terminal ${tabIdCounter}`,
  isLoading: true,
  connectionError: null,
  term: null,
  fitAddon: null,
  ws: null,
  initialized: false,
  el: null,
})

const tabs = ref([createTab()])
const activeTabId = ref(tabs.value[0].id)
const activeTab = () => tabs.value.find(t => t.id === activeTabId.value)

const setRef = (el, tab) => { if (el) tab.el = el }

// ── Terminal init ─────────────────────────────────────────
const initTab = async (tab) => {
  if (tab.initialized || !tab.el) return
  tab.initialized = true

  tab.term = new Terminal({
    cursorBlink: true,
    theme: {
      background: '#0f172a',
      foreground: '#f8fafc',
      cursor: '#38bdf8',
      selectionBackground: '#334155',
    },
    fontFamily: 'Menlo, Monaco, "Courier New", monospace',
    fontSize: 13,
    lineHeight: 1.4,
  })

  tab.fitAddon = new FitAddon()
  tab.term.loadAddon(tab.fitAddon)
  tab.term.open(tab.el)

  await nextTick()
  tab.fitAddon.fit()
  tab.term.writeln('Connecting to backend server...')

  try {
    const token = getToken(activeServerId.value)
    const wsUrl = getActiveServerUrl().replace(/^http/, 'ws') + '/api/terminal/ws?token=' + (token || '')
    tab.ws = new WebSocket(wsUrl)
    tab.ws.binaryType = 'arraybuffer'

    tab.ws.onopen = () => {
      tab.isLoading = false
      tab.term.clear()
      tab.term.focus()
    }

    tab.ws.onmessage = (evt) => {
      if (evt.data instanceof ArrayBuffer) {
        tab.term.write(new Uint8Array(evt.data))
      } else {
        tab.term.write(evt.data)
      }
    }

    tab.ws.onclose = () => {
      tab.isLoading = false
      tab.term?.writeln('\n\r\x1b[31mConnection Closed.\x1b[0m')
    }

    tab.ws.onerror = () => {
      tab.isLoading = false
      tab.connectionError = 'WebSocket connection failed. Ensure backend is running.'
      tab.term?.writeln('\n\r\x1b[31mConnection Error.\x1b[0m')
    }

    tab.term.onData((data) => {
      if (tab.ws?.readyState === WebSocket.OPEN) tab.ws.send(data)
    })
  } catch (err) {
    tab.isLoading = false
    tab.connectionError = err.message
  }
}

const destroyTab = (tab) => {
  if (tab.ws) { const s = tab.ws; tab.ws = null; s.close() }
  if (tab.term) { tab.term.dispose(); tab.term = null }
  tab.initialized = false
  tab.el = null
}

// ── Tab actions ───────────────────────────────────────────
const addTab = async () => {
  const tab = createTab()
  tabs.value.push(tab)
  activeTabId.value = tab.id
  await nextTick()
  setTimeout(() => initTab(tab), 200)
}

const closeTab = (tabId) => {
  const idx = tabs.value.findIndex(t => t.id === tabId)
  if (idx === -1) return
  destroyTab(tabs.value[idx])
  tabs.value.splice(idx, 1)
  if (tabs.value.length === 0) { emit('close'); return }
  activeTabId.value = tabs.value[Math.min(idx, tabs.value.length - 1)].id
}

const switchTab = async (tabId) => {
  activeTabId.value = tabId
  await nextTick()
  const tab = activeTab()
  if (!tab) return
  if (!tab.initialized) {
    setTimeout(() => initTab(tab), 150)
  } else {
    tab.fitAddon?.fit()
    tab.term?.focus()
  }
}

// ── Watch visible prop — init saat modal dibuka ───────────
watch(() => props.visible, async (val) => {
  if (val) {
    await nextTick()
    setTimeout(() => {
      const tab = activeTab()
      if (tab && !tab.initialized) initTab(tab)
      else { tab?.fitAddon?.fit(); tab?.term?.focus() }
    }, 150)
  }
})

const handleResize = () => {
  if (props.visible) activeTab()?.fitAddon?.fit()
}

onMounted(() => {
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  tabs.value.forEach(destroyTab)
})
</script>

<template>
  <!-- Modal Overlay — hanya tampil jika visible prop true -->
  <Teleport to="body">
    <div
      v-if="visible"
      class="fixed inset-0 bg-slate-900/80 z-[60] flex items-center justify-center p-4 backdrop-blur-sm"
    >
      <div class="bg-slate-900 rounded-xl w-full max-w-5xl h-[82vh] flex flex-col overflow-hidden shadow-2xl border border-slate-700">

        <!-- Tab Bar -->
        <div class="flex items-stretch bg-slate-800 border-b border-slate-700 overflow-x-auto shrink-0">

          <div
            v-for="tab in tabs"
            :key="tab.id"
            @click="switchTab(tab.id)"
            class="flex items-center gap-2 px-3 py-2.5 text-sm font-medium cursor-pointer select-none shrink-0 border-r border-slate-700 transition-colors"
            :class="activeTabId === tab.id
              ? 'bg-slate-900 text-white border-t-2 border-t-blue-500'
              : 'text-slate-400 hover:text-slate-200 hover:bg-slate-700/60'"
          >
            <TerminalIcon class="w-3.5 h-3.5 shrink-0" />
            <span class="max-w-[100px] truncate">{{ tab.label }}</span>
            <button
              @click.stop="closeTab(tab.id)"
              class="ml-1 p-0.5 rounded hover:bg-slate-600 text-slate-500 hover:text-red-400 transition-colors"
              title="Close tab"
            >
              <X class="w-3 h-3" />
            </button>
          </div>

          <!-- New Tab -->
          <button
            @click="addTab"
            class="flex items-center justify-center px-3 py-2.5 text-slate-400 hover:text-white hover:bg-slate-700/60 transition-colors shrink-0"
            title="New terminal tab"
          >
            <Plus class="w-4 h-4" />
          </button>

          <div class="flex-1" />

          <!-- Close modal -->
          <button
            @click="$emit('close')"
            class="flex items-center justify-center px-3 py-2.5 text-slate-400 hover:text-red-400 hover:bg-slate-700/60 transition-colors shrink-0 border-l border-slate-700"
            title="Close terminal"
          >
            <X class="w-4 h-4" />
          </button>
        </div>

        <!-- Terminal Panels -->
        <div class="relative flex-1 overflow-hidden bg-slate-900">
          <div
            v-for="tab in tabs"
            :key="tab.id"
            class="absolute inset-0 p-2"
            :class="activeTabId === tab.id ? '' : 'invisible pointer-events-none'"
          >
            <!-- Loading -->
            <div v-if="tab.isLoading && !tab.connectionError"
              class="absolute inset-0 flex flex-col items-center justify-center text-slate-400 z-10 bg-slate-900">
              <Loader2 class="w-8 h-8 animate-spin mb-3 text-blue-500" />
              <p class="text-sm">Initiating PTY Session...</p>
            </div>

            <!-- Error -->
            <div v-if="tab.connectionError"
              class="absolute inset-0 flex flex-col items-center justify-center z-10 bg-slate-900/90 p-6 text-center">
              <div class="bg-red-500/10 border border-red-500/50 rounded-lg p-5 max-w-md">
                <h3 class="text-red-400 font-bold mb-2">Connection Failed</h3>
                <p class="text-red-300 text-sm">{{ tab.connectionError }}</p>
                <button @click="$emit('close')" class="mt-4 btn-secondary btn-sm">Close Terminal</button>
              </div>
            </div>

            <!-- xterm container -->
            <div :ref="el => setRef(el, tab)" class="w-full h-full" />
          </div>
        </div>

      </div>
    </div>
  </Teleport>
</template>

<style>
.xterm-viewport::-webkit-scrollbar { width: 8px; }
.xterm-viewport::-webkit-scrollbar-track { background: #0f172a; }
.xterm-viewport::-webkit-scrollbar-thumb { background: #334155; border-radius: 4px; }
.xterm-viewport::-webkit-scrollbar-thumb:hover { background: #475569; }
</style>
