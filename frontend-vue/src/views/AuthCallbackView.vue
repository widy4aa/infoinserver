<script setup>
import { onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '../stores/authStore'
import { useServerStore } from '../stores/serverStore'

const router = useRouter()
const route  = useRoute()
const { setSession, fetchAndUpdateRole, canWriteConfig, getToken } = useAuthStore()
const { loadConfigFromServer } = useServerStore()

onMounted(async () => {
  const error = route.query.error
  if (error) {
    router.replace(`/login?error=${encodeURIComponent(error)}`)
    return
  }

  const token    = route.query.token
  const username = route.query.user
  const name     = route.query.name
  const avatar   = route.query.avatar

  if (!token || !username) {
    router.replace('/login?error=missing_params')
    return
  }

  // 1. Simpan session dasar dulu (role default: slave)
  setSession({
    token:    String(token),
    username: String(username),
    name:     String(name || username),
    avatar:   String(avatar || ''),
  })

  // 2. Fetch role & allowedGroups dari server, update session
  await fetchAndUpdateRole()

  // 3. Load config global dengan canWrite sesuai role
  await loadConfigFromServer(String(token), canWriteConfig.value)

  // 4. Redirect ke Home
  router.replace('/')
})
</script>

<template>
  <div class="min-h-screen flex items-center justify-center">
    <div class="text-center">
      <div class="w-8 h-8 border-2 border-brand-500 border-t-transparent rounded-full animate-spin mx-auto mb-3"></div>
      <p class="text-sm text-slate-500">Signing you in...</p>
    </div>
  </div>
</template>
