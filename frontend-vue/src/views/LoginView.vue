<script setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { Moon, Sun } from 'lucide-vue-next'
import { useThemeStore } from '../stores/themeStore'
import LoginPanel from '../components/LoginPanel.vue'
import FeatureHighlight from '../components/FeatureHighlight.vue'

const route = useRoute()
const { isDark, toggleDark } = useThemeStore()

// Parse error dari query param ?error=
const errorMessage = computed(() => {
  const err = route.query.error
  if (!err) return null
  const messages = {
    access_denied: 'GitHub access was denied. Please try again.',
    missing_params: 'Authentication failed. Missing required parameters.',
  }
  return messages[err] || 'Authentication failed. Please try again.'
})

// Gradient + dot overlay
const bgStyle = computed(() => {
  if (isDark.value) {
    return {
      backgroundImage: [
        'radial-gradient(circle at 15% 15%, #1d4ed8 0%, transparent 45%)',
        'radial-gradient(circle at 85% 80%, #0369a1 0%, transparent 40%)',
        'radial-gradient(circle at 70% 20%, #1e3a5f 0%, transparent 35%)',
        'radial-gradient(circle, rgba(30,41,59,0.6) 1.5px, transparent 1.5px)',
      ].join(', '),
      backgroundSize: 'auto, auto, auto, 48px 48px',
    }
  }
  return {
    backgroundImage: [
      'radial-gradient(circle at 15% 15%, #93c5fd 0%, transparent 45%)',
      'radial-gradient(circle at 85% 80%, #bae6fd 0%, transparent 40%)',
      'radial-gradient(circle at 70% 20%, #bfdbfe 0%, transparent 35%)',
      'radial-gradient(circle, rgba(148,163,184,0.5) 1.5px, transparent 1.5px)',
    ].join(', '),
    backgroundSize: 'auto, auto, auto, 48px 48px',
  }
})
</script>

<template>
  <div class="min-h-screen relative overflow-hidden"
       :class="isDark ? 'bg-slate-950' : 'bg-slate-50'">

    <!-- Layer 1: bg.jpg — kanan, tanpa blur langsung -->
    <div
      class="absolute inset-0"
      style="
        background-image: url('/bg.jpg');
        background-size: auto 100%;
        background-position: right center;
        background-repeat: no-repeat;
      "
    />

    <!-- Layer 2: backdrop-blur + overlay warna -->
    <div
      class="absolute inset-0"
      :class="isDark ? 'bg-slate-950/70' : 'bg-white/60'"
      style="backdrop-filter: blur(2px); -webkit-backdrop-filter: blur(2px);"
    />

    <!-- Layer 3: gradient + dots -->
    <div class="absolute inset-0" :style="bgStyle" />

    <!-- Layer 4: konten -->
    <div class="relative z-10 min-h-screen flex items-center justify-center p-8">

      <!-- Dark mode toggle -->
      <button
        @click="toggleDark"
        class="absolute top-4 right-4 w-9 h-9 flex items-center justify-center rounded-lg transition-all duration-150
               text-slate-500 hover:text-slate-700 hover:bg-black/5
               dark:text-slate-400 dark:hover:text-white dark:hover:bg-white/10"
        :title="isDark ? 'Switch to Light Mode' : 'Switch to Dark Mode'"
      >
        <Sun v-if="isDark" class="w-4 h-4" />
        <Moon v-else class="w-4 h-4" />
      </button>

      <!-- Card split-screen -->
      <div
        class="w-full max-w-6xl min-h-[580px] rounded-2xl shadow-2xl overflow-hidden
               grid grid-cols-1 md:grid-cols-2
               ring-1 ring-black/5 dark:ring-white/5"
      >
        <LoginPanel :error="errorMessage" />
        <FeatureHighlight
          title="mas dashboardnya jangan di apa2in loh yah 😹😹😹"
          link-href="#"
        />
      </div>

    </div>
  </div>
</template>
