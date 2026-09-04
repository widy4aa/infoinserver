// src/stores/authStore.js
// Menyimpan sesi GitHub OAuth (terpisah dari JWT Linux per-server)
// Disimpan di localStorage agar sesi tetap ada saat halaman di-refresh

import { ref, computed } from 'vue'

const SESSION_KEY = 'github-session'

// Baca langsung dari localStorage — tidak bergantung Vue reactivity
const readSession = () => {
  try {
    const raw = localStorage.getItem(SESSION_KEY)
    if (!raw || raw === 'null') return null
    return JSON.parse(raw)
  } catch { return null }
}

// State reaktif — diinisialisasi dari localStorage saat module pertama kali di-load
const githubSession = ref(readSession())

// Fungsi sync — aman dipanggil di router guard (sebelum Vue app mount)
export const isLoggedInSync = () => !!readSession()?.token

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
    const data = { token, username, name, avatar }
    localStorage.setItem(SESSION_KEY, JSON.stringify(data))
    githubSession.value = data
  }

  const logout = () => {
    localStorage.removeItem(SESSION_KEY)
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
