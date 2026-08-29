<script setup>
import { ref, computed } from 'vue'
import { Lock, User, Loader2, AlertCircle, Server } from 'lucide-vue-next'
import { useServerStore } from '../stores/serverStore'
import { useThemeStore } from '../stores/themeStore'

const props = defineProps({
  server: { type: Object, required: true }
})
const emit = defineEmits(['success'])

const { setToken } = useServerStore()
const { isDark } = useThemeStore()

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

const overlayClass = computed(() => isDark.value ? 'bg-slate-950/80' : 'bg-slate-900/70')
const modalBgClass = computed(() => isDark.value ? 'bg-slate-800' : 'bg-white')
const headerClass = computed(() => isDark.value ? 'bg-slate-900 border-slate-800' : 'bg-slate-900')
const formBgClass = computed(() => isDark.value ? 'bg-slate-800' : 'bg-white')
const labelClass = computed(() => isDark.value ? 'text-slate-400' : 'text-slate-600')
const inputIconClass = computed(() => isDark.value ? 'text-slate-500' : 'text-slate-400')
const errorBgClass = computed(() => isDark.value ? 'bg-red-900/30 border-red-800 text-red-300' : 'bg-red-50 border-red-200 text-red-700')
const helpTextClass = computed(() => isDark.value ? 'text-slate-500' : 'text-slate-400')
</script>

<template>
  <div class="fixed inset-0 backdrop-blur-sm z-[80] flex items-center justify-center p-4" :class="overlayClass">
    <div class="rounded-2xl shadow-2xl w-full max-w-sm overflow-hidden" :class="modalBgClass">

      <!-- Header -->
      <div class="px-6 py-5 flex items-center gap-3 border-b" :class="headerClass">
        <div class="w-9 h-9 rounded-lg bg-blue-600 flex items-center justify-center shrink-0">
          <Lock class="w-4 h-4 text-white" />
        </div>
        <div class="min-w-0">
          <h2 class="text-white font-semibold text-sm leading-tight">Login to Server</h2>
          <div class="text-slate-400 text-xs font-mono truncate mt-0.5">{{ server.name }} · {{ server.url }}</div>
        </div>
      </div>

      <!-- Form -->
      <div class="p-6 space-y-4" :class="formBgClass">
        <p class="text-sm" :class="labelClass">
          Enter your Linux OS credentials for this server.
        </p>

        <!-- Error -->
        <div v-if="error" class="flex items-start gap-2 rounded-lg p-3" :class="errorBgClass">
          <AlertCircle class="w-4 h-4 shrink-0 mt-0.5" />
          <span>{{ error }}</span>
        </div>

        <!-- Username -->
        <div class="space-y-1">
          <label class="text-xs font-semibold uppercase tracking-wider" :class="labelClass">Username</label>
          <div class="relative">
            <User class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 pointer-events-none" :class="inputIconClass" />
            <input
              v-model="username"
              type="text"
              placeholder="e.g. root, ubuntu, widy"
              class="input-field !pl-9"
              autocomplete="username"
              @keydown="onKeydown"
              :disabled="isLoading"
            />
          </div>
        </div>

        <!-- Password -->
        <div class="space-y-1">
          <label class="text-xs font-semibold uppercase tracking-wider" :class="labelClass">Password</label>
          <div class="relative">
            <Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 pointer-events-none" :class="inputIconClass" />
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
          class="btn-primary w-full justify-center"
          :disabled="isLoading"
        >
          <Loader2 v-if="isLoading" class="w-4 h-4 animate-spin" />
          <Lock v-else class="w-4 h-4" />
          {{ isLoading ? 'Authenticating...' : 'Login' }}
        </button>

        <p class="text-center text-xs" :class="helpTextClass">
          Credentials are verified by the server via PAM
        </p>
      </div>
    </div>
  </div>
</template>