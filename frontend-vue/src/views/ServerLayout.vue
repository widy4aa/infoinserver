<script setup>
import { useRoute } from 'vue-router'
import { useServerStore } from '../stores/serverStore'
import { onMounted } from 'vue'
import NativeTerminal from '../components/NativeTerminal.vue'
import { ref } from 'vue'
import { ArrowLeft, Terminal, LayoutDashboard, ShieldCheck, Box, FolderTree, Settings, Cloud } from 'lucide-vue-next'

const route = useRoute()
const { setActiveServer, servers } = useServerStore()
const currentServer = ref(null)
const showTerminal = ref(false)

onMounted(() => {
  const sid = route.params.id
  setActiveServer(sid)
  currentServer.value = servers.value.find(s => s.id === sid)
})
</script>

<template>
  <div v-if="currentServer" class="space-y-6">
    <!-- Server Context Header with Navigation -->
    <div class="bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden">
      <!-- Top info bar -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 border-b border-slate-100 bg-slate-50/50">
        <div class="flex items-center gap-4">
          <RouterLink to="/" class="p-2 bg-white border border-slate-200 hover:bg-slate-100 text-slate-600 rounded-lg transition-colors shadow-sm">
            <ArrowLeft class="w-5 h-5" />
          </RouterLink>
          <div>
            <h2 class="font-bold text-lg text-slate-800 leading-tight">{{ currentServer.name }}</h2>
            <div class="text-xs text-slate-500 font-mono">{{ currentServer.url }}</div>
          </div>
        </div>
        
        <button @click="showTerminal = true" class="btn-primary">
          <Terminal class="w-4 h-4" /> Root Terminal
        </button>
      </div>

      <!-- Feature Tabs inside the server card -->
      <div class="px-2 pb-0 pt-2 bg-white overflow-x-auto">
        <nav class="flex space-x-1 sm:space-x-4 min-w-max pb-[2px]" aria-label="Tabs">
          <RouterLink :to="`/server/${currentServer.id}/dashboard`" class="tab-item" active-class="tab-active">
            <LayoutDashboard class="w-4 h-4" /> System
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/ports`" class="tab-item" active-class="tab-active">
            <ShieldCheck class="w-4 h-4" /> Network & Security
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/podman`" class="tab-item" active-class="tab-active">
            <Box class="w-4 h-4" /> Containers
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/cloudflare`" class="tab-item" active-class="tab-active">
            <Cloud class="w-4 h-4" /> Cloudflare
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/files`" class="tab-item" active-class="tab-active">
            <FolderTree class="w-4 h-4" /> File Explorer
          </RouterLink>
          <RouterLink :to="`/server/${currentServer.id}/settings`" class="tab-item" active-class="tab-active">
            <Settings class="w-4 h-4" /> Config
          </RouterLink>
        </nav>
      </div>
    </div>

    <!-- Inject sub-views based on route -->
    <div class="pt-2">
      <RouterView :key="route.fullPath" />
    </div>

    <!-- Terminal Modal -->
    <NativeTerminal v-if="showTerminal" @close="showTerminal = false" />
  </div>
  <div v-else class="text-center py-12 text-slate-500 flex flex-col items-center justify-center">
    <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600 mb-4"></div>
    Loading server context...
  </div>
</template>