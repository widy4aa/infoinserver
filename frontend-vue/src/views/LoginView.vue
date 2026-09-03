<script setup>
import { useThemeStore } from '../stores/themeStore'

const { isDark } = useThemeStore()

const loginWithGithub = () => {
  window.location.href = '/api/auth/github'
}
</script>

<template>
  <div class="min-h-screen flex items-center justify-center px-4" :class="isDark ? 'bg-slate-950' : 'bg-slate-100'">
    <div class="w-full max-w-sm">

      <!-- Logo & Title -->
      <div class="text-center mb-8">
        <div class="flex justify-center mb-4">
          <img src="/server-icon-blue.svg" alt="InfoIn Server" class="w-16 h-16" />
        </div>
        <h1 class="text-2xl font-bold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">InfoIn Server</h1>
        <p class="text-sm mt-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
          Your Linux server. Fully visible. Fully in control.
        </p>
      </div>

      <!-- Card -->
      <div class="rounded-2xl shadow-lg overflow-hidden" :class="isDark ? 'bg-slate-800 border border-slate-700' : 'bg-white border border-slate-200'">

        <!-- Header -->
        <div class="px-6 py-5 border-b" :class="isDark ? 'border-slate-700 bg-slate-900' : 'border-slate-100 bg-slate-50'">
          <h2 class="font-semibold text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-700'">Sign in to continue</h2>
          <p class="text-xs mt-0.5" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
            Authenticate with your GitHub account to access the dashboard.
          </p>
        </div>

        <!-- Body -->
        <div class="p-6">
          <button
            @click="loginWithGithub"
            class="w-full flex items-center justify-center gap-3 px-4 py-3 rounded-xl font-semibold text-sm transition-all duration-150"
            :class="isDark
              ? 'bg-white text-slate-900 hover:bg-slate-100'
              : 'bg-slate-900 text-white hover:bg-slate-700'"
          >
            <!-- GitHub Icon SVG -->
            <svg class="w-5 h-5 shrink-0" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 0C5.374 0 0 5.373 0 12c0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23A11.509 11.509 0 0112 5.803c1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576C20.566 21.797 24 17.3 24 12c0-6.627-5.373-12-12-12z"/>
            </svg>
            Continue with GitHub
          </button>

          <p class="text-center text-xs mt-4" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
            Only your GitHub identity is verified here.<br/>
            Server access still requires Linux credentials.
          </p>
        </div>
      </div>

      <!-- Footer -->
      <p class="text-center text-xs mt-6" :class="isDark ? 'text-slate-600' : 'text-slate-400'">
        InfoIn Server &mdash; Self-hosted &amp; Open Source
      </p>
    </div>
  </div>
</template>
