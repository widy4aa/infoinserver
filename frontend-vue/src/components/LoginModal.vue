<script setup>
import { ref } from 'vue'
import { Lock, User, Loader2, AlertCircle, ArrowLeft } from 'lucide-vue-next'
import { useRouter } from 'vue-router'
import { useServerStore } from '../stores/serverStore'
import { useThemeStore } from '../stores/themeStore'

const props = defineProps({
  server: { type: Object, required: true }
})
const emit = defineEmits(['success'])

const { setToken } = useServerStore()
const { isDark } = useThemeStore()
const router = useRouter()

const username = ref('')
const password = ref('')
const isLoading = ref(false)
const error = ref(null)

const handleLogin = async () => {
  if (!username.value || !password.value) {
    error.value = 'Username and password are required'
    return
  }

  isLoading.value = true
  error.value = null

  try {
    const res = await fetch(`${props.server.url}/api/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        username: username.value.trim(),
        password: password.value
      })
    })

    const data = await res.json()

    if (res.ok) {
      setToken(props.server.id, data.token, data.username)
      password.value = ''
      emit('success')
    } else {
      error.value = data.error || 'Authentication failed'
    }
  } catch (e) {
    error.value = 'Cannot connect to server. Check if backend is running.'
  } finally {
    isLoading.value = false
  }
}

const onKeydown = (e) => {
  if (e.key === 'Enter') handleLogin()
}
</script>

<template>
  <div class="fixed inset-0 backdrop-blur-sm z-[80] flex items-center justify-center p-4"
       :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">

    <div class="w-full max-w-sm overflow-hidden rounded-2xl animate-fade-slide-up"
         :class="isDark ? 'bg-slate-900 border border-slate-800' : 'bg-white border border-slate-200'"
         style="box-shadow: var(--shadow-modal)">

      <!-- Header -->
      <div class="px-5 py-4 flex items-center gap-3 border-b"
           :class="isDark ? 'border-slate-800' : 'border-slate-100'">

        <!-- Back button -->
        <button @click="router.push('/')"
          class="w-8 h-8 flex items-center justify-center rounded-lg transition-all duration-150 shrink-0"
          :class="isDark
            ? 'text-slate-500 hover:text-slate-200 hover:bg-slate-800'
            : 'text-slate-400 hover:text-slate-700 hover:bg-slate-100'"
          title="Back to Home">
          <ArrowLeft class="w-4 h-4" />
        </button>

        <!-- Brand icon -->
        <div class="w-8 h-8 rounded-lg bg-cyan-500 flex items-center justify-center shrink-0">
          <svg viewBox="0 0 16 16" fill="none" class="w-4 h-4 text-white" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <polyline points="2,5 6,8 2,11" />
            <line x1="8" y1="11" x2="14" y2="11" />
          </svg>
        </div>

        <!-- Server name -->
        <div class="flex-1 min-w-0">
          <div class="text-sm font-semibold truncate"
               :class="isDark ? 'text-slate-100' : 'text-slate-800'">
            {{ server.name }}
          </div>
          <div class="text-[11px] font-mono truncate mt-0.5"
               :class="isDark ? 'text-slate-500' : 'text-slate-400'">
            {{ server.url }}
          </div>
        </div>
      </div>

      <!-- Form -->
      <div class="p-6 space-y-4">

        <!-- Error -->
        <div v-if="error"
          class="flex items-start gap-2 rounded-lg px-3 py-2.5 text-xs border"
          :class="isDark
            ? 'bg-red-900/20 border-red-800/50 text-red-400'
            : 'bg-red-50 border-red-200 text-red-600'">
          <AlertCircle class="w-3.5 h-3.5 shrink-0 mt-0.5" />
          <span>{{ error }}</span>
        </div>

        <!-- Username -->
        <div class="space-y-1.5">
          <label class="text-[10px] font-semibold uppercase tracking-[0.08em]"
                 :class="isDark ? 'text-slate-500' : 'text-slate-400'">
            Username
          </label>
          <div class="relative">
            <User class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 pointer-events-none"
                  :class="isDark ? 'text-slate-600' : 'text-slate-400'" />
            <input
              v-model="username"
              type="text"
              placeholder="root, ubuntu, widy..."
              class="input-field !pl-9"
              autocomplete="username"
              @keydown="onKeydown"
              :disabled="isLoading"
            />
          </div>
        </div>

        <!-- Password -->
        <div class="space-y-1.5">
          <label class="text-[10px] font-semibold uppercase tracking-[0.08em]"
                 :class="isDark ? 'text-slate-500' : 'text-slate-400'">
            Password
          </label>
          <div class="relative">
            <Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 pointer-events-none"
                  :class="isDark ? 'text-slate-600' : 'text-slate-400'" />
            <input
              v-model="password"
              type="password"
              placeholder="OS user password"
              class="input-field !pl-9"
              autocomplete="current-password"
              @keydown="onKeydown"
              :disabled="isLoading"
            />
          </div>
        </div>

        <!-- Submit -->
        <button
          @click="handleLogin"
          :disabled="isLoading"
          class="btn-primary w-full justify-center"
        >
          <Loader2 v-if="isLoading" class="w-4 h-4 animate-spin" />
          <Lock v-else class="w-4 h-4" />
          {{ isLoading ? 'Authenticating...' : 'Login' }}
        </button>

      </div>
    </div>
  </div>
</template>
