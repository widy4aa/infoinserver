<script setup>
import { ref, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { X, Loader2, Terminal as TerminalIcon, Plus } from 'lucide-vue-next'
import { useServerStore } from '../stores/serverStore'
import { isTokenExpired } from '../composables/useApi'
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
  // initialized = xterm sudah di-mount ke DOM dan WS sudah connect
  initialized: false,
  // el = referensi DOM container xterm
  el: null,
  // pending = sedang antri init (untuk hindari double-init)
  pending: false,
})

const tabs = ref([createTab()])
const activeTabId = ref(tabs.value[0].id)
const activeTab = () => tabs.value.find(t => t.id === activeTabId.value)

// setRef dipanggil Vue setiap render — update el dan trigger init jika perlu
const setRef = (el, tab) => {
  if (!el) return
  tab.el = el
  // Jika tab ini aktif dan belum init, jadwalkan init
  if (!tab.initialized && !tab.pending && activeTabId.value === tab.id) {
    scheduleInit(tab)
  }
}

// ── scheduleInit: defer init agar DOM pasti sudah visible ──
const scheduleInit = (tab) => {
  if (tab.initialized || tab.pending) return
  tab.pending = true
  // requestAnimationFrame: pastikan browser sudah paint element ke layar
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      if (tab.el && !tab.initialized) {
        initTab(tab)
      } else {
        tab.pending = false
      }
    })
  })
}

// ── Terminal init ─────────────────────────────────────────
const initTab = async (tab) => {
  if (tab.initialized || !tab.el) {
    tab.pending = false
    return
  }
  tab.initialized = true
  tab.pending = false

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
    // Sembunyikan input saat pengguna mengetik (tidak perlu echo lokal)
    disableStdin: false,
  })

  tab.fitAddon = new FitAddon()
  tab.term.loadAddon(tab.fitAddon)
  tab.term.open(tab.el)

  await nextTick()
  tab.fitAddon.fit()
  tab.term.writeln('\x1b[2mConnecting...\x1b[0m')

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
      // Backend sudah handle semua noise (password prompt, sudo hint, dll)
      // dan mengirim ANSI clear sebelum prompt — langsung render semua output
      if (evt.data instanceof ArrayBuffer) {
        tab.term.write(new Uint8Array(evt.data))
      } else {
        tab.term.write(evt.data)
      }
    }

    tab.ws.onclose = () => {
      tab.isLoading = false
      tab.term?.writeln('\r\n\x1b[31mConnection closed.\x1b[0m')
      const currentToken = getToken(activeServerId.value)
      if (isTokenExpired(currentToken, 0)) {
        window.dispatchEvent(new CustomEvent('auth:expired', {
          detail: { serverId: activeServerId.value }
        }))
      }
    }

    tab.ws.onerror = () => {
      clearTimeout(readyTimeout)
      tab.isLoading = false
      const currentToken = getToken(activeServerId.value)
      if (isTokenExpired(currentToken, 0)) {
        tab.connectionError = 'Session expired. Please login again.'
        window.dispatchEvent(new CustomEvent('auth:expired', {
          detail: { serverId: activeServerId.value }
        }))
      } else {
        tab.connectionError = 'WebSocket connection failed. Ensure backend is running.'
      }
      tab.term?.writeln('\r\n\x1b[31mConnection error.\x1b[0m')
    }

    tab.term.onData((data) => {
      if (tab.ws?.readyState === WebSocket.OPEN) tab.ws.send(data)
    })

  } catch (err) {
    tab.isLoading = false
    tab.connectionError = err.message
    tab.initialized = false // allow retry
  }
}

const destroyTab = (tab) => {
  if (tab.ws) { const s = tab.ws; tab.ws = null; s.close() }
  if (tab.term) { tab.term.dispose(); tab.term = null }
  tab.initialized = false
  tab.pending = false
  tab.el = null
}

// ── Tab actions ───────────────────────────────────────────
const addTab = async () => {
  const tab = createTab()
  tabs.value.push(tab)
  activeTabId.value = tab.id
  // nextTick menunggu Vue render, lalu scheduleInit akan dipanggil via setRef
  await nextTick()
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
    // Element sudah ada di DOM (visible), langsung jadwalkan init
    if (tab.el) {
      scheduleInit(tab)
    }
    // Jika el belum ada, setRef akan trigger scheduleInit saat element mount
  } else {
    // Sudah init — fit ulang karena ukuran mungkin berubah saat invisible
    requestAnimationFrame(() => {
      tab.fitAddon?.fit()
      tab.term?.focus()
    })
  }
}

// ── Watch visible prop ────────────────────────────────────
watch(() => props.visible, async (val) => {
  if (val) {
    await nextTick()
    const tab = activeTab()
    if (!tab) return
    if (!tab.initialized) {
      if (tab.el) scheduleInit(tab)
      // else: setRef akan handle saat DOM mount
    } else {
      requestAnimationFrame(() => {
        tab.fitAddon?.fit()
        tab.term?.focus()
      })
    }
  }
})

// ── Resize handler ────────────────────────────────────────
const handleResize = () => {
  if (props.visible) {
    requestAnimationFrame(() => activeTab()?.fitAddon?.fit())
  }
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
        <!-- Gunakan v-show bukan invisible agar xterm bisa measure DOM size dengan benar -->
        <div class="relative flex-1 overflow-hidden bg-slate-900">
          <div
            v-for="tab in tabs"
            :key="tab.id"
            v-show="activeTabId === tab.id"
            class="absolute inset-0 p-2"
          >
            <!-- Loading overlay -->
            <div v-if="tab.isLoading && !tab.connectionError"
              class="absolute inset-0 flex flex-col items-center justify-center text-slate-400 z-10 bg-slate-900">
              <Loader2 class="w-8 h-8 animate-spin mb-3 text-blue-500" />
              <p class="text-sm">Initiating PTY Session...</p>
            </div>

            <!-- Error overlay -->
            <div v-if="tab.connectionError"
              class="absolute inset-0 flex flex-col items-center justify-center z-10 bg-slate-900/90 p-6 text-center">
              <div class="bg-red-500/10 border border-red-500/50 rounded-lg p-5 max-w-md">
                <h3 class="text-red-400 font-bold mb-2">Connection Failed</h3>
                <p class="text-red-300 text-sm">{{ tab.connectionError }}</p>
                <button @click="$emit('close')" class="mt-4 btn-secondary btn-sm">Close Terminal</button>
              </div>
            </div>

            <!-- xterm mount point — selalu ada di DOM agar setRef bisa set el -->
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
