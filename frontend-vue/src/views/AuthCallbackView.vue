<script setup>
import { onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '../stores/authStore'

const router = useRouter()
const route = useRoute()
const { setSession } = useAuthStore()

onMounted(() => {
  const error = route.query.error
  if (error) {
    router.replace(`/login?error=${encodeURIComponent(error)}`)
    return
  }

  const token = route.query.token
  const username = route.query.user
  const name = route.query.name
  const avatar = route.query.avatar

  if (!token || !username) {
    router.replace('/login?error=missing_params')
    return
  }

  // Simpan session ke localStorage via authStore
  setSession({
    token: String(token),
    username: String(username),
    name: String(name || username),
    avatar: String(avatar || ''),
  })

  // Redirect ke Home
  router.replace('/')
})
</script>

<template>
  <div class="min-h-screen flex items-center justify-center">
    <div class="text-center">
      <div class="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin mx-auto mb-3"></div>
      <p class="text-sm text-slate-500">Signing you in...</p>
    </div>
  </div>
</template>
