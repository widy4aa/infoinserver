<script setup>
import { useRoute } from 'vue-router'
import { useServerStore } from '../stores/serverStore'
import { onMounted, onUnmounted, ref, computed } from 'vue'
import NativeTerminal from '../components/NativeTerminal.vue'
import LoginModal from '../components/LoginModal.vue'
import AddUserForm from '../components/AddUserForm.vue'
import { ArrowLeft, Terminal, LayoutDashboard, ShieldCheck, Box, FolderTree, Settings, Cloud, User, Activity, AlertCircle, Users, PowerSquare, ScrollText, Clock, Download, Server, ChevronDown, Plus, LogOut, Check } from 'lucide-vue-next'
import { getDistroIcon } from '../utils/distro.js'
import { useThemeStore } from '../stores/themeStore'

const { isDark } = useThemeStore()

const route = useRoute()
const { setActiveServer, servers, isAuthenticated, getUsername, getActiveUsername, clearToken, listServerUsers, switchUser, removeUser, addUserToken, activeServerId } = useServerStore()
const currentServer = ref(null)
const showTerminal = ref(false)
const showLogin = ref(false)

// ── User Switcher State ──────────────────────────────────────
const showUserDropdown = ref(false)
const showAddUserModal = ref(false)

const currentUsers = computed(() => {
  if (!currentServer.value?.id) return []
  return listServerUsers(currentServer.value.id)
})

const activeUser = computed(() => {
  if (!currentServer.value?.id) return null
  return getActiveUsername(currentServer.value.id)
})

const handleSwitchUser = (username) => {
  if (!currentServer.value?.id) return
  switchUser(currentServer.value.id, username)
  showUserDropdown.value = false
  // File explorer dan komponen lain akan re-render otomatis karena token berubah
}

const handleRemoveUser = (username) => {
  if (!currentServer.value?.id) return
  const serverId = currentServer.value.id
  const wasActive = activeUser.value === username
  removeUser(serverId, username)
  showUserDropdown.value = false
  // Jika user yang dihapus adalah yang aktif dan tidak ada user tersisa, logout
  if (wasActive && currentUsers.value.length === 0) {
    showLogin.value = true
  }
}

const handleAddUserSuccess = (newUsername, newToken) => {
  if (!currentServer.value?.id) return
  addUserToken(currentServer.value.id, newUsername, newToken)
  // Set user baru sebagai aktif
  switchUser(currentServer.value.id, newUsername)
  showAddUserModal.value = false
}

// Tutup dropdown saat klik di luar
const handleClickOutside = (e) => {
  if (!e.target.closest('.user-switcher-container')) {
    showUserDropdown.value = false
  }
}

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
  window.addEventListener('click', handleClickOutside)
  
  checkPing() // Immediate check
  pingInterval = setInterval(checkPing, 3000)
})

onUnmounted(() => {
  window.removeEventListener('auth:expired', handleAuthExpired)
  window.removeEventListener('click', handleClickOutside)
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
          <!-- Distro Icon di sebelah tombol back -->
          <div class="w-9 h-9 rounded-lg flex items-center justify-center shrink-0 border border-slate-200 dark:border-slate-600 bg-slate-100 dark:bg-slate-700">
            <img v-if="getDistroIcon(currentServer.os_name)" :src="getDistroIcon(currentServer.os_name)" :alt="currentServer.os_name" class="w-6 h-6 object-contain" />
            <Server v-else class="w-5 h-5 text-slate-400" />
          </div>
          <div>
            <div class="flex items-center gap-2">
              <h2 class="font-bold text-lg text-slate-800 dark:text-slate-100 leading-tight">{{ currentServer.name }}</h2>
              <!-- User Switcher Dropdown (menggantikan badge username sederhana) -->
              <div class="relative user-switcher-container" v-if="activeUser">
                <button @click.stop="showUserDropdown = !showUserDropdown"
                  class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-semibold transition-colors cursor-pointer select-none"
                  :class="isDark ? 'bg-blue-900/30 text-blue-300 hover:bg-blue-900/50' : 'bg-blue-100 text-blue-700 hover:bg-blue-200'">
                  <User class="w-3 h-3" />
                  {{ activeUser }}
                  <ChevronDown class="w-3 h-3 transition-transform" :class="showUserDropdown ? 'rotate-180' : ''" />
                </button>

                <!-- Dropdown -->
                <div v-if="showUserDropdown"
                  class="absolute left-0 top-full mt-1.5 rounded-xl shadow-xl border z-[200] overflow-hidden min-w-[200px]"
                  :class="isDark ? 'bg-slate-800 border-slate-700' : 'bg-white border-slate-200'">
                  <!-- Header -->
                  <div class="px-3 py-2 border-b text-[10px] font-bold uppercase tracking-wider text-slate-500"
                    :class="isDark ? 'border-slate-700' : 'border-slate-100'">
                    Switch User
                  </div>
                  <!-- User List -->
                  <div class="py-1">
                    <button v-for="username in currentUsers" :key="username"
                      @click="handleSwitchUser(username)"
                      class="w-full flex items-center justify-between gap-3 px-3 py-2 text-sm transition-colors text-left"
                      :class="isDark ? 'hover:bg-slate-700' : 'hover:bg-slate-50'">
                      <div class="flex items-center gap-2 min-w-0">
                        <div class="w-6 h-6 rounded-full flex items-center justify-center shrink-0 text-[10px] font-bold"
                          :class="username === activeUser
                            ? 'bg-blue-500 text-white'
                            : (isDark ? 'bg-slate-600 text-slate-300' : 'bg-slate-200 text-slate-600')">
                          {{ username.charAt(0).toUpperCase() }}
                        </div>
                        <span class="truncate font-medium" :class="isDark ? 'text-slate-200' : 'text-slate-700'">
                          {{ username }}
                        </span>
                      </div>
                      <div class="flex items-center gap-1.5 shrink-0">
                        <Check v-if="username === activeUser" class="w-3.5 h-3.5 text-blue-500" />
                        <button v-else @click.stop="handleRemoveUser(username)"
                          class="p-0.5 rounded opacity-0 group-hover:opacity-100 transition-opacity text-slate-400 hover:text-red-500"
                          title="Remove user">
                          <LogOut class="w-3 h-3" />
                        </button>
                      </div>
                    </button>
                  </div>
                  <!-- Divider + Add User -->
                  <div class="border-t" :class="isDark ? 'border-slate-700' : 'border-slate-100'">
                    <button @click="showAddUserModal = true; showUserDropdown = false"
                      class="w-full flex items-center gap-2 px-3 py-2 text-xs font-semibold transition-colors"
                      :class="isDark ? 'text-brand-400 hover:bg-slate-700' : 'text-brand-600 hover:bg-slate-50'">
                      <Plus class="w-3.5 h-3.5" />
                      Add Another User
                    </button>
                  </div>
                </div>
              </div>
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
          <button @click="showTerminal = true"
            class="w-9 h-9 rounded-lg bg-brand-600 hover:bg-brand-700 dark:bg-brand-500 dark:hover:bg-brand-400 text-white flex items-center justify-center transition-colors shadow-sm shrink-0"
            title="Root Terminal">
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

    <!-- Login Modal (untuk autentikasi pertama kali / session expired) -->
    <LoginModal
      v-if="showLogin"
      :server="currentServer"
      @success="onLoginSuccess"
    />

    <!-- Add User Modal (untuk tambah user baru ke switcher) -->
    <Teleport to="body">
      <div v-if="showAddUserModal"
        class="fixed inset-0 z-[200] backdrop-blur-sm flex items-center justify-center p-4"
        :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/60'">
        <div class="rounded-2xl shadow-2xl w-full max-w-sm overflow-hidden"
          :class="isDark ? 'bg-slate-800' : 'bg-white'">

          <!-- Header -->
          <div class="px-6 py-5 flex items-center gap-3 border-b"
            :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-900'">
            <div class="w-9 h-9 rounded-lg bg-brand-600 flex items-center justify-center shrink-0">
              <Plus class="w-4 h-4 text-white" />
            </div>
            <div class="min-w-0">
              <h2 class="text-white font-semibold text-sm leading-tight">Add Another User</h2>
              <div class="text-slate-400 text-xs font-mono truncate mt-0.5">{{ currentServer?.name }} · {{ currentServer?.url }}</div>
            </div>
          </div>

          <!-- Form -->
          <AddUserForm
            :server="currentServer"
            @success="handleAddUserSuccess"
            @cancel="showAddUserModal = false"
          />
        </div>
      </div>
    </Teleport>
  </div>

  <div v-else class="text-center py-12 text-slate-500 dark:text-slate-400 flex flex-col items-center justify-center">
    <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600 mb-4"></div>
    Loading server context...
  </div>
</template>