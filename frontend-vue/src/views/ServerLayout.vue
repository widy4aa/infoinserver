<script setup>
import { useRoute } from 'vue-router'
import { useServerStore } from '../stores/serverStore'
import { onMounted, onUnmounted, ref, computed } from 'vue'
import NativeTerminal from '../components/NativeTerminal.vue'
import LoginModal from '../components/LoginModal.vue'
import AddUserForm from '../components/AddUserForm.vue'
import { ArrowLeft, Terminal, LayoutDashboard, ShieldCheck, Box, FolderTree, Settings, Cloud, User, Activity, AlertCircle, Users, PowerSquare, ScrollText, Clock, Download, Server, ChevronDown, Plus, LogOut, Check, Loader2 } from 'lucide-vue-next'
import { getDistroIcon } from '../utils/distro.js'
import { useThemeStore } from '../stores/themeStore'

const { isDark } = useThemeStore()

const route = useRoute()
const { setActiveServer, servers, isAuthenticated, getUsername, getActiveUsername, clearToken, listServerUsers, switchUser, removeUser, addUserToken, activeServerId } = useServerStore()
const currentServer = ref(null)
const showTerminal = ref(false)
const showLogin = ref(false)

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
}

const handleRemoveUser = (username) => {
  if (!currentServer.value?.id) return
  const serverId = currentServer.value.id
  const wasActive = activeUser.value === username
  removeUser(serverId, username)
  showUserDropdown.value = false
  if (wasActive && currentUsers.value.length === 0) {
    showLogin.value = true
  }
}

const handleAddUserSuccess = (newUsername, newToken) => {
  if (!currentServer.value?.id) return
  addUserToken(currentServer.value.id, newUsername, newToken)
  switchUser(currentServer.value.id, newUsername)
  showAddUserModal.value = false
}

const handleClickOutside = (e) => {
  if (!e.target.closest('.user-switcher-container')) {
    showUserDropdown.value = false
  }
}

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
    await fetch(`${url}/api/ping`, { signal: controller.signal })
    clearTimeout(timeoutId)
    pingMs.value = Math.round(performance.now() - startTime)
  } catch (e) {
    pingMs.value = -1
  }
}

const pingClass = computed(() => {
  if (pingMs.value === null) return 'text-slate-400'
  if (pingMs.value === -1)   return 'text-red-400'
  if (pingMs.value < 100)    return 'text-emerald-400'
  if (pingMs.value < 300)    return 'text-amber-400'
  return 'text-orange-400'
})

const pingDotClass = computed(() => {
  if (pingMs.value === null) return 'bg-slate-400'
  if (pingMs.value === -1)   return 'bg-red-500'
  if (pingMs.value < 100)    return 'bg-emerald-500'
  if (pingMs.value < 300)    return 'bg-amber-500'
  return 'bg-orange-500'
})

const checkAuth = () => {
  const sid = currentServer.value?.id
  if (!sid) return
  showLogin.value = !isAuthenticated(sid)
}

const onLoginSuccess = () => { showLogin.value = false }

const handleAuthExpired = (e) => {
  if (e.detail?.serverId === currentServer.value?.id) {
    showLogin.value = true
    showTerminal.value = false
  }
}

onMounted(() => {
  const sid = route.params.id
  setActiveServer(sid)
  currentServer.value = servers.value.find(s => s.id === sid)
  checkAuth()
  window.addEventListener('auth:expired', handleAuthExpired)
  window.addEventListener('click', handleClickOutside)
  checkPing()
  pingInterval = setInterval(checkPing, 3000)
  if (!isAuthenticated(sid)) showLogin.value = true
})

onUnmounted(() => {
  window.removeEventListener('auth:expired', handleAuthExpired)
  window.removeEventListener('click', handleClickOutside)
  if (pingInterval) clearInterval(pingInterval)
})

const navItems = computed(() => [
  { to: `/server/${currentServer.value?.id}/dashboard`,  icon: LayoutDashboard, label: 'System' },
  { to: `/server/${currentServer.value?.id}/updates`,    icon: Download,        label: 'Updates' },
  { to: `/server/${currentServer.value?.id}/services`,   icon: PowerSquare,     label: 'Services' },
  { to: `/server/${currentServer.value?.id}/files`,      icon: FolderTree,      label: 'Files' },
  { to: `/server/${currentServer.value?.id}/containers`, icon: Box,             label: 'Containers' },
  { to: `/server/${currentServer.value?.id}/users`,      icon: Users,           label: 'Users' },
  { to: `/server/${currentServer.value?.id}/ports`,      icon: ShieldCheck,     label: 'Network' },
  { to: `/server/${currentServer.value?.id}/cloudflare`, icon: Cloud,           label: 'Cloudflare' },
  { to: `/server/${currentServer.value?.id}/cron`,       icon: Clock,           label: 'Cron' },
  { to: `/server/${currentServer.value?.id}/syslogs`,    icon: ScrollText,      label: 'Syslogs' },
  { to: `/server/${currentServer.value?.id}/logs`,       icon: AlertCircle,     label: 'Alerts' },
  { to: `/server/${currentServer.value?.id}/settings`,   icon: Settings,        label: 'Config' },
])
</script>

<template>
  <div v-if="currentServer" class="flex gap-4 min-h-[calc(100vh-5rem)]">

    <!-- ── Sidebar kiri ── -->
    <aside class="w-64 shrink-0 flex flex-col gap-3 sticky top-24 h-fit">

      <!-- Server identity card — liquid glass -->
      <div class="rounded-2xl overflow-hidden border"
           :class="isDark ? 'border-white/8' : 'border-white/30'"
           :style="isDark
             ? 'background: rgba(15,23,42,0.55); backdrop-filter: blur(20px) saturate(160%); -webkit-backdrop-filter: blur(20px) saturate(160%); box-shadow: 0 4px 24px rgba(0,0,0,0.24), inset 0 1px 0 rgba(255,255,255,0.06);'
             : 'background: rgba(255,255,255,0.55); backdrop-filter: blur(20px) saturate(180%); -webkit-backdrop-filter: blur(20px) saturate(180%); box-shadow: 0 4px 24px rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.8);'">

        <!-- Distro banner — full width, square-ish -->
        <div class="relative w-full h-28 flex items-center justify-center border-b"
             :class="isDark ? 'border-white/6 bg-slate-900/40' : 'border-black/5 bg-slate-100/60'">
          <!-- Back button — pojok kiri atas -->
          <RouterLink to="/" @click="handleGoHome"
            class="absolute top-2.5 left-2.5 w-7 h-7 flex items-center justify-center rounded-lg transition-all z-10"
            :class="isDark ? 'text-slate-400 hover:text-slate-200 hover:bg-white/10' : 'text-slate-500 hover:text-slate-700 hover:bg-black/8'"
            title="Back to Home">
            <ArrowLeft class="w-3.5 h-3.5" />
          </RouterLink>

          <!-- Distro icon besar -->
          <img v-if="getDistroIcon(currentServer.os_name)"
            :src="getDistroIcon(currentServer.os_name)"
            :alt="currentServer.os_name"
            class="w-16 h-16 object-contain drop-shadow-sm" />
          <Server v-else class="w-16 h-16 text-slate-300 dark:text-slate-600" />

          <!-- Ping badge — pojok kanan atas -->
          <div class="absolute top-2.5 right-2.5 flex items-center gap-1 px-1.5 py-0.5 rounded-md text-[10px] font-mono border"
               :class="isDark ? 'bg-slate-900/60 border-white/8' : 'bg-white/60 border-black/8'">
            <div class="w-1.5 h-1.5 rounded-full" :class="pingDotClass"></div>
            <span :class="pingClass">{{ pingMs === null ? '...' : pingMs === -1 ? 'timeout' : pingMs + 'ms' }}</span>
          </div>
        </div>

        <!-- Info + controls -->
        <div class="p-4 space-y-3">
          <!-- Server name + url -->
          <div>
            <div class="font-bold text-sm leading-tight truncate"
                 :class="isDark ? 'text-slate-100' : 'text-slate-800'">
              {{ currentServer.name }}
            </div>
            <div class="text-[10px] font-mono truncate mt-0.5"
                 :class="isDark ? 'text-slate-500' : 'text-slate-400'">
              {{ currentServer.url }}
            </div>
          </div>

          <!-- User switcher -->
          <div class="relative user-switcher-container" v-if="activeUser">
            <button @click.stop="showUserDropdown = !showUserDropdown"
              class="w-full flex items-center gap-1.5 px-2 py-1.5 rounded-lg text-xs font-semibold transition-colors"
              :class="isDark ? 'bg-cyan-900/30 text-cyan-300 hover:bg-cyan-900/50' : 'bg-cyan-100/80 text-cyan-700 hover:bg-cyan-100'">
              <User class="w-3 h-3 shrink-0" />
              <span class="truncate flex-1 text-left">{{ activeUser }}</span>
              <ChevronDown class="w-3 h-3 shrink-0 transition-transform" :class="showUserDropdown ? 'rotate-180' : ''" />
            </button>

            <div v-if="showUserDropdown"
              class="absolute left-0 top-full mt-1.5 w-full rounded-xl border z-[200] overflow-hidden"
              :class="isDark ? 'bg-slate-800 border-slate-700' : 'bg-white border-slate-200'"
              style="box-shadow: var(--shadow-dropdown)">
              <div class="px-3 py-2 border-b text-[10px] font-bold uppercase tracking-wider text-slate-500"
                :class="isDark ? 'border-slate-700' : 'border-slate-100'">Switch User</div>
              <div class="py-1">
                <button v-for="username in currentUsers" :key="username"
                  @click="handleSwitchUser(username)"
                  class="w-full flex items-center justify-between gap-2 px-3 py-1.5 text-xs transition-colors text-left"
                  :class="isDark ? 'hover:bg-slate-700' : 'hover:bg-slate-50'">
                  <div class="flex items-center gap-2 min-w-0">
                    <div class="w-5 h-5 rounded-full flex items-center justify-center shrink-0 text-[9px] font-bold"
                      :class="username === activeUser ? 'bg-cyan-500 text-white' : (isDark ? 'bg-slate-600 text-slate-300' : 'bg-slate-200 text-slate-600')">
                      {{ username.charAt(0).toUpperCase() }}
                    </div>
                    <span class="truncate font-medium" :class="isDark ? 'text-slate-200' : 'text-slate-700'">{{ username }}</span>
                  </div>
                  <Check v-if="username === activeUser" class="w-3 h-3 text-cyan-500 shrink-0" />
                  <button v-else @click.stop="handleRemoveUser(username)"
                    class="p-0.5 rounded opacity-0 group-hover:opacity-100 transition-opacity text-slate-400 hover:text-red-500"
                    title="Remove user">
                    <LogOut class="w-3 h-3" />
                  </button>
                </button>
              </div>
              <div class="border-t" :class="isDark ? 'border-slate-700' : 'border-slate-100'">
                <button @click="showAddUserModal = true; showUserDropdown = false"
                  class="w-full flex items-center gap-2 px-3 py-2 text-xs font-semibold transition-colors"
                  :class="isDark ? 'text-brand-400 hover:bg-slate-700' : 'text-brand-600 hover:bg-slate-50'">
                  <Plus class="w-3 h-3" /> Add Another User
                </button>
              </div>
            </div>
          </div>

          <!-- Terminal button -->
          <button @click="showTerminal = true"
            class="w-full flex items-center justify-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors
                   bg-brand-500 hover:bg-brand-600 text-white"
            title="Open Terminal">
            <Terminal class="w-3.5 h-3.5" /> Terminal
          </button>
        </div>
      </div>

      <!-- Navigation card — liquid glass -->
      <nav class="rounded-2xl border overflow-hidden"
           :class="isDark ? 'border-white/8' : 'border-white/30'"
           :style="isDark
             ? 'background: rgba(15,23,42,0.55); backdrop-filter: blur(20px) saturate(160%); -webkit-backdrop-filter: blur(20px) saturate(160%); box-shadow: 0 4px 24px rgba(0,0,0,0.24), inset 0 1px 0 rgba(255,255,255,0.06);'
             : 'background: rgba(255,255,255,0.55); backdrop-filter: blur(20px) saturate(180%); -webkit-backdrop-filter: blur(20px) saturate(180%); box-shadow: 0 4px 24px rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.8);'">
        <RouterLink
          v-for="item in navItems" :key="item.to"
          :to="item.to"
          class="sidebar-nav-item"
          active-class="sidebar-nav-active">
          <component :is="item.icon" class="w-4 h-4 shrink-0" />
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>
    </aside>

    <!-- ── Main content ── -->
    <div class="flex-1 min-w-0 pb-8">
      <div v-if="!showLogin">
        <RouterView :key="route.fullPath" />
      </div>
    </div>

    <!-- Terminal Modal -->
    <NativeTerminal :visible="showTerminal" @close="showTerminal = false" />

    <!-- Login Modal -->
    <LoginModal v-if="showLogin" :server="currentServer" @success="onLoginSuccess" />

    <!-- Add User Modal -->
    <Teleport to="body">
      <div v-if="showAddUserModal"
        class="fixed inset-0 z-[200] backdrop-blur-sm flex items-center justify-center p-4"
        :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/60'">
        <div class="rounded-2xl w-full max-w-sm overflow-hidden"
          :class="isDark ? 'bg-slate-800' : 'bg-white'"
          style="box-shadow: var(--shadow-modal)">
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
          <AddUserForm :server="currentServer" @success="handleAddUserSuccess" @cancel="showAddUserModal = false" />
        </div>
      </div>
    </Teleport>
  </div>

  <div v-else class="text-center py-12 text-slate-500 dark:text-slate-400 flex flex-col items-center justify-center gap-3">
    <Loader2 class="w-6 h-6 animate-spin text-brand-500" />
    <span class="text-sm">Loading server context...</span>
  </div>
</template>

<style scoped>
.sidebar-nav-item {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  padding: 0.5rem 0.875rem;
  font-size: 0.8125rem;
  font-weight: 500;
  transition: all 0.15s;
  color: v-bind("isDark ? '#94a3b8' : '#64748b'");
  border-left: 2px solid transparent;
}

.sidebar-nav-item:hover {
  color: v-bind("isDark ? '#e2e8f0' : '#1e293b'");
  background: v-bind("isDark ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.04)'");
}

.sidebar-nav-active {
  color: v-bind("isDark ? '#67e8f9' : '#0891b2'") !important;
  background: v-bind("isDark ? 'rgba(6,182,212,0.1)' : 'rgba(6,182,212,0.08)'") !important;
  border-left-color: v-bind("isDark ? '#22d3ee' : '#06b6d4'") !important;
}
</style>
