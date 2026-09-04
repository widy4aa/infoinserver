<script setup>
import { ref } from 'vue'
import { Loader2 } from 'lucide-vue-next'

defineProps({
  error: { type: String, default: null }
})

const isLoading = ref(false)

const loginWithGithub = () => {
  isLoading.value = true
  window.location.href = '/api/auth/github'
}
</script>

<template>
  <div class="flex flex-col justify-center h-full p-8 md:p-10 bg-white dark:bg-slate-800">

    <!-- Top section -->
    <div class="flex flex-col gap-8">

      <!-- Brand -->
      <div class="flex items-center gap-2.5">
        <img src="/server-icon-blue.svg" alt="InfoIn Server" class="w-7 h-7 object-contain" />
        <span class="text-sm font-semibold text-slate-800 dark:text-slate-100 tracking-tight">
          infoinserver
        </span>
      </div>

      <!-- Heading -->
      <div class="flex flex-col gap-1.5">
        <h1 class="text-2xl font-bold text-slate-900 dark:text-slate-100">
          Log in
        </h1>
        <p class="text-sm text-slate-500 dark:text-slate-400">
          Sign in to continue to your dashboard.
        </p>
      </div>

      <!-- Error banner -->
      <div
        v-if="error"
        role="alert"
        class="flex items-start gap-2 rounded-lg border px-3 py-2.5 text-sm
               bg-red-50 border-red-200 text-red-600
               dark:bg-red-900/20 dark:border-red-800 dark:text-red-400"
      >
        <svg class="w-4 h-4 mt-0.5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
        <span>{{ error }}</span>
      </div>

      <!-- GitHub button -->
      <button
        @click="loginWithGithub"
        :disabled="isLoading"
        aria-label="Sign in with GitHub"
        class="w-full flex items-center justify-center gap-2.5 px-4 py-2.5 rounded-lg
               text-sm font-semibold transition-all duration-150 select-none
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2
               active:scale-[0.97] disabled:opacity-50 disabled:cursor-not-allowed
               bg-slate-900 text-white hover:bg-slate-700
               dark:bg-white dark:text-slate-900 dark:hover:bg-slate-100
               dark:focus-visible:ring-offset-slate-800"
      >
        <Loader2 v-if="isLoading" class="w-4 h-4 animate-spin shrink-0" />
        <svg v-else class="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 0C5.374 0 0 5.373 0 12c0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23A11.509 11.509 0 0112 5.803c1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576C20.566 21.797 24 17.3 24 12c0-6.627-5.373-12-12-12z"/>
        </svg>
        {{ isLoading ? 'Redirecting...' : 'Sign in with GitHub' }}
      </button>

      <!-- Divider note -->
      <p class="text-xs text-slate-400 dark:text-slate-500 text-center leading-relaxed">
        Only your GitHub identity is verified here.<br/>
        Linux credentials are required for server access.
      </p>
    </div>

  </div>
</template>
