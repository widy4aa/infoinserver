<script setup>
import { ref, onMounted } from 'vue'
import { RouterLink } from 'vue-router'
import { useServerStore } from '../stores/serverStore'
import { Server, Plus, LayoutDashboard, ShieldCheck, Box, FolderTree, Settings, Trash2, Cloud } from 'lucide-vue-next'

const { servers, removeServer } = useServerStore()
const serverHostnames = ref({})

const fetchHostname = async (server) => {
  try {
    const res = await fetch(`${server.url}/api/system`)
    if (res.ok) {
      const data = await res.json()
      serverHostnames.value[server.id] = data.hostname || server.name
    }
  } catch (e) {
    console.error(`Failed to fetch hostname for ${server.url}`, e)
  }
}

onMounted(() => {
  servers.value.forEach(s => fetchHostname(s))
})
</script>

<template>
  <div class="space-y-6">
    <div class="flex justify-between items-center">
      <h2 class="text-2xl font-bold text-slate-800">Your Servers</h2>
      <RouterLink to="/settings" class="btn-primary">
        <Plus class="w-4 h-4" /> Add Server
      </RouterLink>
    </div>

    <div v-if="servers.length === 0" class="text-center py-12 bg-white rounded-xl border border-slate-200 border-dashed">
      <Server class="w-12 h-12 text-slate-300 mx-auto mb-3" />
      <h3 class="text-lg font-medium text-slate-900">No servers configured</h3>
      <p class="text-slate-500 mt-1 mb-4">Add your first backend server to start monitoring.</p>
      <RouterLink to="/settings" class="btn-primary"><Plus class="w-4 h-4" /> Add Server</RouterLink>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <div v-for="s in servers" :key="s.id" class="bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden hover:shadow-md transition-shadow flex flex-col group">
        
        <!-- Header Card (Clickable to enter server) -->
        <RouterLink :to="`/server/${s.id}/dashboard`" class="p-5 border-b border-slate-100 bg-slate-50 flex items-start gap-4 hover:bg-brand-50 transition-colors flex-1 cursor-pointer">
          <div class="w-12 h-12 rounded-xl bg-brand-100 flex items-center justify-center text-brand-600 shrink-0 shadow-sm group-hover:bg-brand-600 group-hover:text-white transition-colors">
            <Server class="w-6 h-6" />
          </div>
          <div class="flex-1 min-w-0">
            <!-- Menampilkan Hostname OS jika didapat, jika gagal tampilkan nama alias -->
            <h3 class="font-bold text-slate-800 text-lg leading-tight truncate" :title="serverHostnames[s.id] || s.name">
              {{ serverHostnames[s.id] || s.name }}
            </h3>
            <div class="text-xs text-slate-500 font-mono mt-1 truncate">{{ s.url }}</div>
          </div>
        </RouterLink>

        <!-- Footer Menu (Quick Links) -->
        <div class="p-2 border-t border-slate-100 bg-white grid grid-cols-5 divide-x divide-slate-100">
          <RouterLink :to="`/server/${s.id}/ports`" class="flex items-center justify-center p-2 text-slate-500 hover:text-brand-600 hover:bg-slate-50 rounded transition-colors" title="Ports & Scan">
            <ShieldCheck class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/podman`" class="flex items-center justify-center p-2 text-slate-500 hover:text-brand-600 hover:bg-slate-50 rounded transition-colors" title="Podman">
            <Box class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/cloudflare`" class="flex items-center justify-center p-2 text-slate-500 hover:text-brand-600 hover:bg-slate-50 rounded transition-colors" title="Cloudflare">
            <Cloud class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/files`" class="flex items-center justify-center p-2 text-slate-500 hover:text-brand-600 hover:bg-slate-50 rounded transition-colors" title="File Explorer">
            <FolderTree class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/settings`" class="flex items-center justify-center p-2 text-slate-500 hover:text-brand-600 hover:bg-slate-50 rounded transition-colors" title="Settings">
            <Settings class="w-4 h-4" />
          </RouterLink>
        </div>

      </div>
    </div>
  </div>
</template>