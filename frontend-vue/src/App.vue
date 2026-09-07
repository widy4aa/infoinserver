<script setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { RouterView, RouterLink, useRouter, useRoute } from 'vue-router'
import { Moon, Sun, LogOut, ChevronDown } from 'lucide-vue-next'
import ToastAlert from './components/ToastAlert.vue'
import { useToastStore } from './stores/toastStore'
import { useThemeStore } from './stores/themeStore'
import { useAuthStore } from './stores/authStore'
// import { useHeartbeat } from './composables/useHeartbeat'

const { showToast } = useToastStore()
const { isDark, toggleDark } = useThemeStore()
const { isLoggedIn, githubUser, logout, getToken } = useAuthStore()
const router = useRouter()
const route = useRoute()
const isFullscreen = computed(() => !!route.meta?.fullscreen)
const hideNav = computed(() => !!route.meta?.hideNav)

// ── Heartbeat (nonaktif sementara — pakai ping di HomeView) ────────
// useHeartbeat(getToken)

// ── Global background style (shared across all pages) ──────────────
const globalBgStyle = computed(() => {
  if (isDark.value) {
    return {
      backgroundColor: '#020617', // slate-950
      backgroundImage: [
        'radial-gradient(circle at 15% 15%, rgba(29,78,216,0.55) 0%, transparent 45%)',
        'radial-gradient(circle at 85% 80%, rgba(3,105,161,0.45) 0%, transparent 40%)',
        'radial-gradient(circle at 70% 20%, rgba(14,116,144,0.35) 0%, transparent 35%)',
        'radial-gradient(circle at 50% 50%, rgba(6,182,212,0.06) 0%, transparent 60%)',
        'radial-gradient(circle, rgba(30,41,59,0.6) 1.5px, transparent 1.5px)',
      ].join(', '),
      backgroundSize: 'auto, auto, auto, auto, 48px 48px',
    }
  }
  return {
    backgroundColor: '#f8fafc', // slate-50
    backgroundImage: [
      'radial-gradient(circle at 15% 15%, rgba(165,243,252,0.8) 0%, transparent 45%)',
      'radial-gradient(circle at 85% 80%, rgba(186,230,253,0.7) 0%, transparent 40%)',
      'radial-gradient(circle at 70% 20%, rgba(207,250,254,0.7) 0%, transparent 35%)',
      'radial-gradient(circle at 50% 50%, rgba(6,182,212,0.08) 0%, transparent 60%)',
      'radial-gradient(circle, rgba(148,163,184,0.4) 1.5px, transparent 1.5px)',
    ].join(', '),
    backgroundSize: 'auto, auto, auto, auto, 48px 48px',
  }
})

// ── Users presence ─────────────────────────────────────────────────
const allUsers = ref([])
const USERS_INTERVAL = 30000

const fetchUsers = async () => {
  try {
    const res = await fetch('/api/auth/github/users')
    if (res.ok) allUsers.value = await res.json()
  } catch {}
}

let usersTimer = null
watch(isLoggedIn, (loggedIn) => {
  if (loggedIn) {
    fetchUsers()
    usersTimer = setInterval(fetchUsers, USERS_INTERVAL)
  } else {
    if (usersTimer) clearInterval(usersTimer)
    allUsers.value = []
  }
}, { immediate: true })
onUnmounted(() => { if (usersTimer) clearInterval(usersTimer) })

// Online duluan, lalu offline — urut last_seen desc
const sortedUsers = computed(() => {
  return [...allUsers.value].sort((a, b) => {
    if (a.online !== b.online) return b.online - a.online
    return b.last_seen - a.last_seen
  })
})

// Maks 3 avatar di navbar, sisanya jadi "+N"
const visibleAvatars = computed(() => sortedUsers.value.slice(0, 3))
const extraCount = computed(() => Math.max(0, sortedUsers.value.length - 3))

// ── Dropdown ───────────────────────────────────────────────────────
const showDropdown = ref(false)

const toggleDropdown = () => { showDropdown.value = !showDropdown.value }

const closeDropdown = (e) => {
  if (!e.target.closest('.presence-dropdown-container')) {
    showDropdown.value = false
  }
}

onMounted(() => window.addEventListener('click', closeDropdown))
onUnmounted(() => window.removeEventListener('click', closeDropdown))

// ── Logout ─────────────────────────────────────────────────────────
const handleLogout = () => {
  showDropdown.value = false
  logout()
  router.push('/login')
}
</script>

<template>
  <div class="min-h-screen flex flex-col relative">

    <!-- Global background layers (semua page kecuali fullscreen) -->
    <template v-if="!isFullscreen">
      <!-- base color + radial gradients + dot grid -->
      <div class="fixed inset-0 -z-10 pointer-events-none"
           :style="globalBgStyle" />
    </template>

    <!-- Top Navigation — hidden on login page (hideNav), absent on fullscreen -->
    <div v-if="!isFullscreen && !hideNav" class="sticky top-0 z-40 px-3 sm:px-4 pt-3">
      <header class="max-w-screen-2xl mx-auto rounded-2xl backdrop-blur-md border
                     bg-white/70 border-slate-200/60
                     dark:bg-slate-900/60 dark:border-slate-700/50"
              style="box-shadow: 0 4px 24px rgba(0,0,0,0.08), 0 1px 0 0 rgba(6,182,212,0.1), inset 0 1px 0 0 rgba(255,255,255,0.1)">
      <div class="px-5 sm:px-6">
        <div class="flex justify-between items-center h-16">
          <RouterLink to="/" class="flex items-center gap-2 hover:opacity-80 transition-opacity">
            <img src="/server-icon-blue.svg" alt="InfoIn Server" class="w-8 h-8 object-contain" />
            <h1 class="text-xl font-bold text-slate-800 dark:text-slate-100 tracking-tight hidden sm:block">infoinserver</h1>
          </RouterLink>

          <div class="flex items-center gap-2">
            <!-- Dark mode toggle -->
            <button @click="toggleDark"
              class="w-9 h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-slate-700 hover:bg-black/5 dark:text-slate-300 dark:hover:text-white dark:hover:bg-white/8 transition-all duration-150"
              :title="isDark ? 'Switch to Light Mode' : 'Switch to Dark Mode'">
              <Sun v-if="isDark" class="w-4 h-4" />
              <Moon v-else class="w-4 h-4" />
            </button>

            <!-- GitHub Presence Dropdown -->
            <div v-if="isLoggedIn && githubUser" class="presence-dropdown-container relative pl-2 border-l border-slate-200 dark:border-slate-700">

              <!-- Trigger: avatar stack + username + chevron -->
              <button @click.stop="toggleDropdown"
                class="flex items-center gap-2 px-2 py-1.5 rounded-lg hover:bg-black/5 dark:hover:bg-white/8 transition-all duration-150">

                <!-- Avatar stack (maks 3) -->
                <div class="flex -space-x-2">
                  <img
                    v-for="u in visibleAvatars" :key="u.username"
                    :src="u.avatar_url"
                    :alt="u.username"
                    :title="u.username"
                    class="w-7 h-7 rounded-full object-cover ring-2 ring-white/80 dark:ring-slate-900/80"
                  />
                  <!-- +N badge -->
                  <div v-if="extraCount > 0"
                    class="w-7 h-7 rounded-full bg-slate-200 dark:bg-slate-700 ring-2 ring-white dark:ring-slate-900 flex items-center justify-center text-[10px] font-bold text-slate-600 dark:text-slate-300">
                    +{{ extraCount }}
                  </div>
                </div>

                <!-- Username aktif -->
                <span class="text-sm font-medium text-slate-700 dark:text-slate-300 hidden sm:block">
                  {{ githubUser.username }}
                </span>
                <ChevronDown class="w-3.5 h-3.5 text-slate-400 transition-transform duration-150"
                  :class="showDropdown ? 'rotate-180' : ''" />
              </button>

              <!-- Dropdown panel -->
              <div v-if="showDropdown"
                class="absolute right-0 top-full mt-2 w-72 rounded-xl border overflow-hidden z-50"
                :class="isDark ? 'bg-slate-800 border-slate-700' : 'bg-white border-slate-200'"
                style="box-shadow: var(--shadow-dropdown)">

                <!-- Daftar users -->
                <div class="max-h-80 overflow-y-auto">
                  <div v-for="u in sortedUsers" :key="u.username"
                    class="flex items-center gap-3 px-4 py-3 border-b last:border-0"
                    :class="[
                      isDark ? 'border-slate-700' : 'border-slate-100',
                      u.username === githubUser.username
                        ? (isDark ? 'bg-slate-700/50' : 'bg-cyan-50')
                        : ''
                    ]">

                    <!-- Avatar + online dot -->
                    <div class="relative shrink-0">
                      <img :src="u.avatar_url" :alt="u.username"
                        class="w-9 h-9 rounded-full object-cover" />
                      <!-- Status dot -->
                      <span class="absolute bottom-0 right-0 w-2.5 h-2.5 rounded-full ring-2 ring-white dark:ring-slate-800"
                        :class="u.online ? 'bg-green-500' : 'bg-slate-400'">
                      </span>
                    </div>

                    <!-- Info -->
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-1.5">
                        <span class="text-sm font-semibold truncate"
                          :class="isDark ? 'text-slate-100' : 'text-slate-800'">
                          {{ u.username }}
                        </span>
                        <!-- "you" badge -->
                        <span v-if="u.username === githubUser.username"
                          class="text-[10px] px-1.5 py-0.5 rounded-full font-medium bg-cyan-100 text-cyan-700 dark:bg-cyan-900/40 dark:text-cyan-400 shrink-0">
                          you
                        </span>
                      </div>
                      <div class="flex items-center gap-1 mt-0.5">
                        <span class="text-xs font-medium"
                          :class="u.online ? 'text-green-500' : 'text-slate-400'">
                          {{ u.online ? 'Online' : 'Offline' }}
                        </span>
                      </div>
                    </div>
                  </div>

                  <!-- Empty state -->
                  <div v-if="sortedUsers.length === 0"
                    class="px-4 py-6 text-center text-sm text-slate-400">
                    No users found
                  </div>
                </div>

                <!-- Footer: Logout -->
                <div class="border-t" :class="isDark ? 'border-slate-700' : 'border-slate-100'">
                  <button @click="handleLogout"
                    class="w-full flex items-center gap-2 px-4 py-3 text-sm font-medium text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors">
                    <LogOut class="w-4 h-4" />
                    Logout
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      </header>
    </div>

    <!-- Main Content -->
    <main :class="isFullscreen || hideNav ? 'flex-1' : 'flex-1 w-full max-w-screen-2xl mx-auto px-3 sm:px-4 py-6'">
      <RouterView />
    </main>

    <!-- Global Toast & Confirm Dialog -->
    <ToastAlert />
  </div>
</template>

<style>
/* Tab navigation styles with dark mode support */
.tab-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  white-space: nowrap;
  padding: 0.5rem 0.5rem;
  border-bottom-width: 2px;
  font-weight: 500;
  font-size: 0.875rem;
  transition-property: color, background-color, border-color;
  transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
  transition-duration: 150ms;
  border-color: transparent;
  color: #64748b;
  margin-bottom: -1px;
  letter-spacing: 0.01em;
}

.dark .tab-item {
  color: #94a3b8;
}

.tab-item:hover {
  color: #334155;
  border-color: #cbd5e1;
}

.dark .tab-item:hover {
  color: #e2e8f0;
  border-color: #475569;
}

.tab-active {
  border-color: #06b6d4 !important;
  color: #0891b2 !important;
}

.dark .tab-active {
  border-color: #22d3ee !important;
  color: #67e8f9 !important;
}
</style>
