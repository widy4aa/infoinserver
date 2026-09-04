// src/composables/useHeartbeat.js
// Kirim heartbeat ke backend setiap 30 detik agar last_seen ter-update
// Dipanggil dari App.vue saat user sudah login GitHub

import { onMounted, onUnmounted } from 'vue'

const HEARTBEAT_INTERVAL = 30000 // 30 detik

export const useHeartbeat = (getToken) => {
  let timer = null

  const sendHeartbeat = async () => {
    const token = getToken()
    if (!token) return
    try {
      await fetch('/api/auth/github/heartbeat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ token }),
      })
    } catch {
      // Abaikan error — tidak perlu alert user jika heartbeat gagal
    }
  }

  onMounted(() => {
    sendHeartbeat() // langsung kirim saat pertama mount
    timer = setInterval(sendHeartbeat, HEARTBEAT_INTERVAL)
  })

  onUnmounted(() => {
    if (timer) clearInterval(timer)
  })
}
