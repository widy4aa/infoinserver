<script setup>
import { ref } from 'vue'
import { Lock, User, Loader2, AlertCircle } from 'lucide-vue-next'
import { useThemeStore } from '../stores/themeStore'

const props = defineProps({
  server: { type: Object, required: true }
})
const emit = defineEmits(['success', 'cancel'])

const { isDark } = useThemeStore()

const username = ref('')
const password = ref('')
const isLoading = ref(false)
const error = ref(null)

const handleAdd = async () => {
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
      // Emit username dan token ke parent (ServerLayout)
      emit('success', data.username, data.token)
      username.value = ''
      password.value = ''
    } else {
      error.value = data.error || 'Authentication failed'
    }
  } catch (e) {
    error.value = 'Cannot connect to server. Check if backend is running.'
  } finally {
    isLoading.value = false
  }
}
</script>

<template>
  <div class="p-6 space-y-4" :class="isDark ? 'bg-slate-800' : 'bg-white'">
    <p class="text-sm" :class="isDark ? 'text-slate-400' : 'text-slate-600'">
      Login with another OS user. Only non-root users with <code class="text-xs bg-slate-100 dark:bg-slate-700 px-1 rounded">sudo</code> privileges are allowed.
    </p>

    <!-- Error -->
    <div v-if="error" class="flex items-start gap-2 rounded-lg p-3 text-sm"
      :class="isDark ? 'bg-red-900/30 border border-red-800 text-red-300' : 'bg-red-50 border border-red-200 text-red-700'">
      <AlertCircle class="w-4 h-4 shrink-0 mt-0.5" />
      <span>{{ error }}</span>
    </div>

    <!-- Username -->
    <div class="space-y-1">
      <label class="text-xs font-semibold uppercase tracking-wider"
        :class="isDark ? 'text-slate-400' : 'text-slate-600'">Username</label>
      <div class="relative">
        <User class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 pointer-events-none"
          :class="isDark ? 'text-slate-500' : 'text-slate-400'" />
        <input
          v-model="username"
          type="text"
          placeholder="e.g. webmaster, deploy"
          class="input-field !pl-9"
          autocomplete="username"
          @keydown.enter="handleAdd"
          :disabled="isLoading"
        />
      </div>
    </div>

    <!-- Password -->
    <div class="space-y-1">
      <label class="text-xs font-semibold uppercase tracking-wider"
        :class="isDark ? 'text-slate-400' : 'text-slate-600'">Password</label>
      <div class="relative">
        <Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 pointer-events-none"
          :class="isDark ? 'text-slate-500' : 'text-slate-400'" />
        <input
          v-model="password"
          type="password"
          placeholder="OS user password"
          class="input-field !pl-9"
          autocomplete="current-password"
          @keydown.enter="handleAdd"
          :disabled="isLoading"
        />
      </div>
    </div>

    <!-- Buttons -->
    <div class="flex gap-3 pt-1">
      <button @click="$emit('cancel')" class="btn-outline flex-1 justify-center" :disabled="isLoading">
        Cancel
      </button>
      <button @click="handleAdd" class="btn-primary flex-1 justify-center" :disabled="isLoading">
        <Loader2 v-if="isLoading" class="w-4 h-4 animate-spin" />
        <User v-else class="w-4 h-4" />
        {{ isLoading ? 'Verifying...' : 'Add User' }}
      </button>
    </div>
  </div>
</template>
