<script setup>
import { useRoute } from 'vue-router'
import { useServerStore } from '../stores/serverStore'
import { onMounted, onUnmounted, ref } from 'vue'
import NativeTerminal from '../components/NativeTerminal.vue'
import LoginModal from '../components/LoginModal.vue'
import { ArrowLeft, Terminal, LayoutDashboard, ShieldCheck, Box, FolderTree, Settings, Cloud, User, Activity, AlertCircle, Users, PowerSquare, ScrollText, Clock } from 'lucide-vue-next'

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
})

onUnmounted(() => {
  window.removeEventListener('auth:expired', handleAuthExpired)
  // Tidak hapus token di sini — supaya refresh tidak logout
})
</script>

<template>
  <div v-if="currentServer" class="space-y-6">
    <!-- Server Context Header with Navigation -->
    <div class="bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden">
      <!-- Top info bar -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 border-b border-slate-100 bg-slate-50/50">
        <div class="flex items-center gap-4">
          <RouterLink to="/" @click="handleGoHome" class="p-2 bg-white border border-slate-200 hover:bg-slate-100 text-slate-600 rounded-lg transition-colors shadow-sm">
            <ArrowLeft class="w-5 h-5" />
          </RouterLink>
          <div>
            <div class="flex items-center gap-2">
              <h2 class="font-bold text-lg text-slate-800 leading-tight">{{ currentServer.name }}</h2>
              <span v-if="getUsername(currentServer.id)" class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-semibold bg-blue-100 text-blue-700">
                <User class="w-3 h-3" />{{ getUsername(currentServer.id) }}
              </span>
            </div>
            <div class="text-xs text-slate-500 font-mono">{{ currentServer.url }}</div>
          </div>
        </div>

        <button @click="showTerminal = true" class="btn-primary">
          <Terminal class="w-4 h-4" /> Root Terminal
        </button>
      </div>

      <!-- Feature Tabs -->
      <div class="px-3 pb-1 pt-2 bg-white border-t border-slate-100">
        <nav class="flex flex-wrap gap-x-2 sm:gap-x-4 gap-y-1" aria-label="Tabs">
          <RouterLink :to="`/server/${currentServer.id}/dashboard`" class="tab-item" active-class="tab-active">
            <LayoutDashboard class="w-4 h-4" /> System
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/speedtest`" class="tab-item" active-class="tab-active">
            <Activity class="w-4 h-4" /> Speedtest
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/services`" class="tab-item" active-class="tab-active">
            <PowerSquare class="w-4 h-4" /> Services
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/files`" class="tab-item" active-class="tab-active">
            <FolderTree class="w-4 h-4" /> File Explorer
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/podman`" class="tab-item" active-class="tab-active">
            <Box class="w-4 h-4" /> Containers
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/users`" class="tab-item" active-class="tab-active">
            <Users class="w-4 h-4" /> Users &amp; Groups
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/ports`" class="tab-item" active-class="tab-active">
            <ShieldCheck class="w-4 h-4" /> Network &amp; Security
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

  <div v-else class="text-center py-12 text-slate-500 flex flex-col items-center justify-center">
    <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600 mb-4"></div>
    Loading server context...
  </div>
</template>
