<script setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useThemeStore } from '../stores/themeStore'
import LoginPanel from '../components/LoginPanel.vue'
import FeatureHighlight from '../components/FeatureHighlight.vue'

const route = useRoute()
const { isDark } = useThemeStore()

const errorMessage = computed(() => {
  const err = route.query.error
  if (!err) return null
  const messages = {
    access_denied: 'GitHub access was denied. Please try again.',
    missing_params: 'Authentication failed. Missing required parameters.',
  }
  return messages[err] || 'Authentication failed. Please try again.'
})
</script>

<template>
  <div class="min-h-screen relative flex items-center justify-center p-8">

    <!-- Layer 1 (paling bawah): bg.jpg kanan, opacity rendah -->
    <div class="absolute inset-0 overflow-hidden">
      <div style="
             position: absolute; inset: -4px;
             background-image: url('/bg.jpg');
             background-size: auto 105%;
             background-position: right center;
             background-repeat: no-repeat;
             filter: blur(3px);
             opacity: 0.1;
           " />
    </div>

    <!-- Layer 2: color overlay lebih tipis -->
    <div class="absolute inset-0"
         :class="isDark ? 'bg-slate-950/50' : 'bg-white/50'" />

    <!-- Layer 3 (paling atas dari bg): gradient dari globalBgStyle App.vue sudah ada di sini via z-index global -->

    <!-- Content -->
    <div class="relative z-10 w-full flex items-center justify-center">

      <!-- Card split-screen -->
      <div
        class="w-full max-w-5xl min-h-[560px] rounded-2xl overflow-hidden
               grid grid-cols-1 md:grid-cols-2
               animate-fade-slide-up"
        style="box-shadow: 0 32px 80px rgba(0,0,0,0.4), 0 0 0 1px rgba(255,255,255,0.07)"
      >
        <LoginPanel :error="errorMessage" />
        <FeatureHighlight />
      </div>

    </div>
  </div>
</template>
