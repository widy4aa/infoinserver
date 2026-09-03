<script setup>
import { RouterView, RouterLink, useRouter } from 'vue-router'
import { Moon, Sun, LogOut } from 'lucide-vue-next'
import ToastAlert from './components/ToastAlert.vue'
import { useToastStore } from './stores/toastStore'
import { useThemeStore } from './stores/themeStore'
import { useAuthStore } from './stores/authStore'

const { showToast } = useToastStore()
const { isDark, toggleDark } = useThemeStore()
const { isLoggedIn, githubUser, logout } = useAuthStore()
const router = useRouter()

const handleLogout = () => {
  logout()
  router.push('/login')
}
</script>

<template>
  <div class="min-h-screen flex flex-col bg-slate-100 dark:bg-slate-950">
    <!-- Top Navigation -->
    <header class="bg-white border-b border-slate-200 sticky top-0 z-40 dark:bg-slate-900 dark:border-slate-800">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="flex justify-between items-center h-16">
          <RouterLink to="/" class="flex items-center gap-2 hover:opacity-80 transition-opacity">
            <img src="/server-icon-blue.svg" alt="InfoIn Server" class="w-8 h-8 object-contain" />
            <h1 class="text-xl font-bold text-slate-800 dark:text-slate-100 tracking-tight hidden sm:block">infoinserver</h1>
          </RouterLink>
          
          <div class="flex items-center gap-2">
            <!-- Dark mode toggle -->
            <button @click="toggleDark"
              class="w-9 h-9 flex items-center justify-center rounded-lg text-slate-500 hover:text-slate-700 hover:bg-slate-100 dark:text-slate-300 dark:hover:text-white dark:hover:bg-slate-800 transition-all duration-150"
              :title="isDark ? 'Switch to Light Mode' : 'Switch to Dark Mode'">
              <Sun v-if="isDark" class="w-4 h-4" />
              <Moon v-else class="w-4 h-4" />
            </button>

            <!-- GitHub User Info + Logout (hanya tampil kalau sudah login) -->
            <template v-if="isLoggedIn && githubUser">
              <div class="flex items-center gap-2 pl-2 border-l border-slate-200 dark:border-slate-700">
                <!-- Avatar -->
                <img
                  v-if="githubUser.avatar"
                  :src="githubUser.avatar"
                  :alt="githubUser.name"
                  class="w-7 h-7 rounded-full object-cover ring-2 ring-slate-200 dark:ring-slate-700"
                />
                <!-- Username -->
                <span class="text-sm font-medium text-slate-700 dark:text-slate-300 hidden sm:block">
                  {{ githubUser.name || githubUser.username }}
                </span>
                <!-- Logout button -->
                <button
                  @click="handleLogout"
                  class="w-8 h-8 flex items-center justify-center rounded-lg text-slate-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-all duration-150"
                  title="Logout GitHub"
                >
                  <LogOut class="w-4 h-4" />
                </button>
              </div>
            </template>
          </div>
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <main class="flex-1 w-full max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
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
  border-color: #3b82f6 !important;
  color: #2563eb !important;
}

.dark .tab-active {
  border-color: #60a5fa !important;
  color: #93c5fd !important;
}
</style>