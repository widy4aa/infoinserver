// src/stores/authStore.js
// Menyimpan sesi GitHub OAuth (terpisah dari JWT Linux per-server)
// Disimpan di localStorage agar sesi tetap ada saat halaman di-refresh
// Key: 'github-session'

import { ref, computed } from 'vue'
import { useStorage } from '@vueuse/core'

const githubSession = useStorage('github-session', null)

export const useAuthStore = () => {
  const isLoggedIn = computed(() => !!githubSession.value?.token)

  const githubUser = computed(() => {
    if (!githubSession.value) return null
    return {
      username: githubSession.value.username,
      name: githubSession.value.name,
      avatar: githubSession.value.avatar,
    }
  })

  const setSession = ({ token, username, name, avatar }) => {
    githubSession.value = { token, username, name, avatar }
  }

  const logout = () => {
    githubSession.value = null
  }

  const getToken = () => githubSession.value?.token || null

  return {
    isLoggedIn,
    githubUser,
    setSession,
    logout,
    getToken,
  }
}
