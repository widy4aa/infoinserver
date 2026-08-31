<script setup>
import { useRoute } from 'vue-router'
import { useServerStore } from '../stores/serverStore'
import { onMounted, onUnmounted, ref } from 'vue'
import NativeTerminal from '../components/NativeTerminal.vue'
import LoginModal from '../components/LoginModal.vue'
import { ArrowLeft, Terminal, LayoutDashboard, ShieldCheck, Box, FolderTree, Settings, Cloud, User, Activity, AlertCircle, Users, PowerSquare, ScrollText, Clock, Download } from 'lucide-vue-next'

const route = useRoute()
const { setActiveServer, servers, isAuthenticated, getUsername, clearToken } = useServerStore()
const currentServer = ref(null)
const showTerminal = ref(false)
const showLogin = ref(false)

// Logout manual — hanya dipanggil saat klik tombol Back ke Home
const handleGoHome = () => {
  if (currentServer.value?.id) {
    clearToken(currentServer.value.id)
  }
}

// ── PING / LATENCY MONITOR ──
const pingMs = ref(null)
let pingInterval = null

const checkPing = async () => {
  if (!currentServer.value?.url) return
  const url = currentServer.value.url.startsWith('http') ? currentServer.value.url : `http://${currentServer.value.url}`
  const startTime = performance.now()
  
  try {
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), 3000)
    
    // Panggil endpoint /api/ping public yang sangat ringan
    await fetch(`${url}/api/ping`, { signal: controller.signal })
    clearTimeout(timeoutId)
    
    const endTime = performance.now()
    pingMs.value = Math.round(endTime - startTime)
  } catch (e) {
    // Jika timeout atau gagal
    pingMs.value = -1
  }
}

const checkAuth = () => {
  const sid = currentServer.value?.id
  if (!sid) return
  if (!isAuthenticated(sid)) {
    showLogin.value = true
  } else {
    showLogin.value = false
  }
}

const onLoginSuccess = () => {
  showLogin.value = false
}

const handleAuthExpired = (e) => {
  if (e.detail?.serverId === currentServer.value?.id) {
    showLogin.value = true
    showTerminal.value = false
  }
}

onMounted(() => {
  const sid = route.params.id
  setActiveServer(sid)
  const server = servers.value.find(s => s.id === sid)
  currentServer.value = server
  checkAuth()
  window.addEventListener('auth:expired', handleAuthExpired)
  
  checkPing() // Immediate check
  pingInterval = setInterval(checkPing, 3000)
})

onUnmounted(() => {
  window.removeEventListener('auth:expired', handleAuthExpired)
  if (pingInterval) clearInterval(pingInterval)
  // Tidak hapus token di sini — supaya refresh tidak logout
})
</script>

<template>
  <div v-if="currentServer" class="space-y-6">
    <!-- Server Context Header with Navigation -->
    <div class="bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden dark:bg-slate-800 dark:border-slate-700">
      <!-- Top info bar -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 border-b border-slate-100 bg-slate-50/50 dark:border-slate-800 dark:bg-slate-900/50">
        <div class="flex items-center gap-4">
          <RouterLink to="/" @click="handleGoHome" class="p-2 bg-white border border-slate-200 hover:bg-slate-100 text-slate-600 rounded-lg transition-colors shadow-sm dark:bg-slate-700 dark:border-slate-600 dark:hover:bg-slate-600 dark:text-slate-300">
            <ArrowLeft class="w-5 h-5" />
          </RouterLink>
          <div>
            <div class="flex items-center gap-2">
              <h2 class="font-bold text-lg text-slate-800 dark:text-slate-100 leading-tight">{{ currentServer.name }}</h2>
              <span v-if="getUsername(currentServer.id)" class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-semibold bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300">
                <User class="w-3 h-3" />{{ getUsername(currentServer.id) }}
              </span>
            </div>
            <div class="text-xs text-slate-500 dark:text-slate-400 font-mono">{{ currentServer.url }}</div>
          </div>
        </div>

        <div class="flex items-center gap-3">
          <!-- Ping Indicator -->
          <div class="flex items-center gap-1.5 px-2 py-1 rounded-md text-[11px] font-mono font-medium border"
               :class="pingMs === null ? 'bg-slate-100 text-slate-500 border-slate-200 dark:bg-slate-800 dark:text-slate-400 dark:border-slate-700' :
                       pingMs === -1 ? 'bg-red-100 text-red-600 border-red-200 dark:bg-red-900/30 dark:text-red-400 dark:border-red-800/50' :
                       pingMs < 100 ? 'bg-emerald-50 text-emerald-600 border-emerald-200 dark:bg-emerald-900/20 dark:text-emerald-400 dark:border-emerald-800/30' :
                       pingMs < 300 ? 'bg-amber-50 text-amber-600 border-amber-200 dark:bg-amber-900/20 dark:text-amber-400 dark:border-amber-800/30' :
                       'bg-orange-50 text-orange-600 border-orange-200 dark:bg-orange-900/20 dark:text-orange-400 dark:border-orange-800/30'">
            <div class="w-2 h-2 rounded-full" 
                 :class="pingMs === null ? 'bg-slate-400' : 
                         pingMs === -1 ? 'bg-red-500' : 
                         pingMs < 100 ? 'bg-emerald-500' : 
                         pingMs < 300 ? 'bg-amber-500' : 
                         'bg-orange-500'"></div>
            <span>{{ pingMs === null ? 'ping...' : pingMs === -1 ? 'timeout' : pingMs + 'ms' }}</span>
          </div>

          <!-- Terminal Button -->
          <button @click="showTerminal = true" class="btn-primary w-9 h-9 p-0 flex items-center justify-center rounded-lg" title="Root Terminal">
            <Terminal class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- Feature Tabs -->
      <div class="px-3 pb-1 pt-2 bg-white border-t border-slate-100 dark:bg-slate-800 dark:border-slate-700">
        <nav class="flex flex-wrap gap-x-2 sm:gap-x-4 gap-y-1" aria-label="Tabs">
          <RouterLink :to="`/server/${currentServer.id}/dashboard`" class="tab-item" active-class="tab-active">
            <LayoutDashboard class="w-4 h-4" /> System
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/updates`" class="tab-item" active-class="tab-active">
            <Download class="w-4 h-4" /> Updates
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/services`" class="tab-item" active-class="tab-active">
            <PowerSquare class="w-4 h-4" /> Services
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/files`" class="tab-item" active-class="tab-active">
            <FolderTree class="w-4 h-4" /> File Explorer
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/containers`" class="tab-item" active-class="tab-active">
            <Box class="w-4 h-4" /> Containers
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/users`" class="tab-item" active-class="tab-active">
            <Users class="w-4 h-4" /> Users & Groups
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/ports`" class="tab-item" active-class="tab-active">
            <ShieldCheck class="w-4 h-4" /> Network & Security
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/cloudflare`" class="tab-item" active-class="tab-active">
            <Cloud class="w-4 h-4" /> Cloudflare
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/cron`" class="tab-item" active-class="tab-active">
            <Clock class="w-4 h-4" /> Cronjobs
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/syslogs`" class="tab-item" active-class="tab-active">
            <ScrollText class="w-4 h-4" /> Syslogs
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/logs`" class="tab-item" active-class="tab-active">
            <AlertCircle class="w-4 h-4" /> Alerts
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/settings`" class="tab-item" active-class="tab-active">
            <Settings class="w-4 h-4" /> Config
          </RouterLink>
        </nav>
      </div>
    </div>

    <!-- Sub-view -->
    <div class="pt-2" v-if="!showLogin">
      <RouterView :key="route.fullPath" />
    </div>

    <!-- Terminal Modal — pakai Teleport ke body, render via visible prop -->
    <NativeTerminal :visible="showTerminal" @close="showTerminal = false" />

    <!-- Login Modal -->
    <LoginModal
      v-if="showLogin"
      :server="currentServer"
      @success="onLoginSuccess"
    />
  </div>

  <div v-else class="text-center py-12 text-slate-500 dark:text-slate-400 flex flex-col items-center justify-center">
    <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600 mb-4"></div>
    Loading server context...
  </div>
</template>