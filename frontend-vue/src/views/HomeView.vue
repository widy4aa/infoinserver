<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useServerStore } from '../stores/serverStore'
import { useThemeStore } from '../stores/themeStore'
import { useToastStore } from '../stores/toastStore'
import { Server, Plus, ShieldCheck, Box, FolderTree, Settings, Cloud, Activity } from 'lucide-vue-next'
import { getDistroIcon, getDistroColorClass } from '../utils/distro.js'

const router = useRouter()
const { servers } = useServerStore()
const { isDark } = useThemeStore()
const { showToast } = useToastStore()

// ── Ping state per server ─────────────────────────────────────────────────────
// { [serverId]: { ms: number | null, online: boolean | null } }
// null = still checking (initial state)
const pingState = ref({})

const PING_TIMEOUT_MS = 5000
const PING_INTERVAL_MS = 10000

const checkPing = async (server) => {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), PING_TIMEOUT_MS)
  const start = performance.now()
  try {
    await fetch(`${server.url}/api/ping`, { signal: controller.signal })
    const ms = Math.round(performance.now() - start)
    pingState.value[server.id] = { ms, online: true }
  } catch {
    pingState.value[server.id] = { ms: null, online: false }
  } finally {
    clearTimeout(timer)
  }
}

let intervals = []

onMounted(() => {
  servers.value.forEach(s => {
    pingState.value[s.id] = { ms: null, online: null }
    checkPing(s)
    const id = setInterval(() => checkPing(s), PING_INTERVAL_MS)
    intervals.push(id)
  })
})

onUnmounted(() => {
  intervals.forEach(clearInterval)
  intervals = []
})

// ── Ping badge helpers ────────────────────────────────────────────────────────
const pingLabel = (id) => {
  const p = pingState.value[id]
  if (!p || p.online === null) return '···'
  if (!p.online) return 'OFFLINE'
  return p.ms + 'ms'
}

const pingDotClass = (id) => {
  const p = pingState.value[id]
  if (!p || p.online === null) return 'bg-slate-400'
  if (!p.online) return 'bg-red-500'
  if (p.ms < 100) return 'bg-emerald-500'
  if (p.ms < 300) return 'bg-amber-500'
  return 'bg-orange-500'
}

const pingTextClass = (id) => {
  const p = pingState.value[id]
  if (!p || p.online === null) return 'text-slate-400'
  if (!p.online) return 'text-red-400'
  if (p.ms < 100) return 'text-emerald-400'
  if (p.ms < 300) return 'text-amber-400'
  return 'text-orange-400'
}

const isOnline = (id) => pingState.value[id]?.online === true
const isChecking = (id) => pingState.value[id]?.online === null
const isOffline = (id) => pingState.value[id]?.online === false

// ── Card click handler ────────────────────────────────────────────────────────
const handleCardClick = (s) => {
  if (isOffline(s.id)) {
    showToast(
      'Server Offline',
      `Cannot connect to ${s.url}. Make sure the backend is running.`,
      'error'
    )
    return
  }
  router.push(`/server/${s.id}/dashboard`)
}

// ── Card style ────────────────────────────────────────────────────────────────
const cardStyle = (hover = false) => {
  if (isDark.value) {
    return hover
      ? 'background: rgba(15,23,42,0.30); backdrop-filter: blur(20px) saturate(160%); -webkit-backdrop-filter: blur(20px) saturate(160%); box-shadow: 0 8px 32px rgba(0,0,0,0.32), 0 1px 0 0 rgba(255,255,255,0.08) inset;'
      : 'background: rgba(15,23,42,0.20); backdrop-filter: blur(20px) saturate(160%); -webkit-backdrop-filter: blur(20px) saturate(160%); box-shadow: 0 4px 24px rgba(0,0,0,0.24), 0 1px 0 0 rgba(255,255,255,0.06) inset;'
  }
  return hover
    ? 'background: rgba(255,255,255,0.40); backdrop-filter: blur(20px) saturate(180%); -webkit-backdrop-filter: blur(20px) saturate(180%); box-shadow: 0 8px 32px rgba(0,0,0,0.12), 0 1px 0 0 rgba(255,255,255,0.8) inset;'
    : 'background: rgba(255,255,255,0.25); backdrop-filter: blur(20px) saturate(180%); -webkit-backdrop-filter: blur(20px) saturate(180%); box-shadow: 0 4px 24px rgba(0,0,0,0.08), 0 1px 0 0 rgba(255,255,255,0.6) inset;'
}

const cardStyleOffline = () => {
  if (isDark.value) {
    return 'background: rgba(15,23,42,0.03); backdrop-filter: blur(8px) saturate(60%) grayscale(10%); -webkit-backdrop-filter: blur(8px) saturate(60%) grayscale(10%); box-shadow: none; opacity: 0.35;'
  }
  return 'background: rgba(255,255,255,0.03); backdrop-filter: blur(8px) saturate(60%) grayscale(10%); -webkit-backdrop-filter: blur(8px) saturate(60%) grayscale(10%); box-shadow: none; opacity: 0.35;'
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex justify-between items-center animate-fade-slide-up">
      <h2 class="text-2xl font-bold text-slate-800 dark:text-slate-100 tracking-tight">Your Servers</h2>
      <RouterLink to="/settings" class="btn-primary">
        <Plus class="w-4 h-4" /> Add Server
      </RouterLink>
    </div>

    <div v-if="servers.length === 0" class="text-center py-16 bg-white dark:bg-slate-800 rounded-xl border border-slate-200 dark:border-slate-700 border-dashed animate-fade-slide-up">
      <Server class="w-10 h-10 text-slate-300 dark:text-slate-600 mx-auto mb-3" />
      <h3 class="text-base font-semibold text-slate-900 dark:text-slate-100">No servers configured</h3>
      <p class="text-sm text-slate-500 dark:text-slate-400 mt-1 mb-4">Add your first backend server to start monitoring.</p>
      <RouterLink to="/settings" class="btn-primary"><Plus class="w-4 h-4" /> Add Server</RouterLink>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <div v-for="(s, i) in servers" :key="s.id"
        class="rounded-2xl overflow-hidden flex flex-col group stagger-item transition-all duration-500 hover:-translate-y-0.5"
        :style="`--i: ${i}; ${isOffline(s.id) ? cardStyleOffline() : cardStyle()}`"
        @mouseenter="e => e.currentTarget.setAttribute('style', `--i: ${i}; ${isOffline(s.id) ? cardStyleOffline() : cardStyle(true)}`)"
        @mouseleave="e => e.currentTarget.setAttribute('style', `--i: ${i}; ${isOffline(s.id) ? cardStyleOffline() : cardStyle(false)}`)"
      >

        <!-- Header Card (Clickable to enter server) -->
        <div
          class="p-5 border-b flex items-start gap-4 flex-1 relative transition-colors border-white/20 dark:border-white/8"
          :style="isOffline(s.id) ? 'filter: grayscale(100%)' : ''"
          :class="isOffline(s.id)
            ? 'cursor-not-allowed'
            : 'cursor-pointer hover:bg-white/20 dark:hover:bg-white/5'"
          @click="handleCardClick(s)"
        >
          <!-- Distro Icon -->
          <div class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0 shadow-sm group-hover:opacity-90 transition-opacity"
               :class="s.os_name ? (getDistroColorClass(s.os_name) || 'bg-slate-100 dark:bg-slate-700') : 'bg-brand-100 dark:bg-brand-900/30'">
            <img v-if="getDistroIcon(s.os_name)" :src="getDistroIcon(s.os_name)" :alt="s.os_name" class="w-7 h-7 object-contain" />
            <Server v-else class="w-6 h-6 text-brand-600 dark:text-brand-400 group-hover:text-brand-700" />
          </div>

          <!-- Server info -->
          <div class="flex-1 min-w-0">
            <h3 class="font-bold text-slate-800 dark:text-slate-100 text-lg leading-tight truncate" :title="s.name">
              {{ s.name }}
            </h3>
            <div class="text-xs text-slate-500 dark:text-slate-400 font-mono mt-1 truncate">{{ s.url }}</div>
            <div v-if="s.os_name" class="text-[10px] text-slate-400 dark:text-slate-500 mt-0.5 truncate">{{ s.os_name }}</div>
          </div>

          <!-- Ping badge (top-right) -->
          <div class="flex items-center gap-1 shrink-0 self-start">
            <!-- Sonar dot: animate-ping hanya saat online -->
            <div class="relative w-2 h-2 flex items-center justify-center">
              <span v-if="isOnline(s.id)" class="animate-ping absolute inline-flex h-full w-full rounded-full opacity-60"
                :class="pingDotClass(s.id)"></span>
              <span class="relative inline-flex w-1.5 h-1.5 rounded-full" :class="pingDotClass(s.id)"></span>
            </div>
            <span class="text-[10px] font-medium tracking-[0.05em] uppercase tabular-nums transition-colors duration-300"
              :class="pingTextClass(s.id)">
              {{ pingLabel(s.id) }}
            </span>
          </div>
        </div>

        <!-- Footer Menu (Quick Links) -->
        <div class="p-2 border-t grid grid-cols-6 divide-x
                    border-white/20 dark:border-white/8
                    divide-white/20 dark:divide-white/8"
          :style="isOffline(s.id) ? 'filter: grayscale(100%)' : ''"
        >
          <RouterLink :to="`/server/${s.id}/speedtest`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Speedtest">
            <Activity class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/ports`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Ports & Scan">
            <ShieldCheck class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/containers`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Containers">
            <Box class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/cloudflare`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Cloudflare">
            <Cloud class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/files`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="File Explorer">
            <FolderTree class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/settings`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Settings">
            <Settings class="w-4 h-4" />
          </RouterLink>
        </div>

      </div>
    </div>
  </div>
</template>
