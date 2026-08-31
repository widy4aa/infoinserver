<script setup>
import { RouterLink } from 'vue-router'
import { useServerStore } from '../stores/serverStore'
import { Server, Plus, ShieldCheck, Box, FolderTree, Settings, Cloud, Activity } from 'lucide-vue-next'
import { getDistroIcon, getDistroColorClass } from '../utils/distro.js'

const { servers } = useServerStore()
</script>

<template>
  <div class="space-y-6">
    <div class="flex justify-between items-center">
      <h2 class="text-2xl font-bold text-slate-800 dark:text-slate-100">Your Servers</h2>
      <RouterLink to="/settings" class="btn-primary">
        <Plus class="w-4 h-4" /> Add Server
      </RouterLink>
    </div>

    <div v-if="servers.length === 0" class="text-center py-12 bg-white dark:bg-slate-800 rounded-xl border border-slate-200 dark:border-slate-700 border-dashed">
      <Server class="w-12 h-12 text-slate-300 dark:text-slate-600 mx-auto mb-3" />
      <h3 class="text-lg font-medium text-slate-900 dark:text-slate-100">No servers configured</h3>
      <p class="text-slate-500 dark:text-slate-400 mt-1 mb-4">Add your first backend server to start monitoring.</p>
      <RouterLink to="/settings" class="btn-primary"><Plus class="w-4 h-4" /> Add Server</RouterLink>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <div v-for="s in servers" :key="s.id" class="bg-white dark:bg-slate-800 rounded-xl border border-slate-200 dark:border-slate-700 shadow-sm overflow-hidden hover:shadow-md transition-shadow flex flex-col group">
        
        <!-- Header Card (Clickable to enter server) -->
        <RouterLink :to="`/server/${s.id}/dashboard`" class="p-5 border-b border-slate-100 dark:border-slate-700 bg-slate-50 dark:bg-slate-800/50 flex items-start gap-4 hover:bg-brand-50 dark:hover:bg-slate-700 transition-colors flex-1 cursor-pointer">
          <!-- Distro Icon (jika ada os_name) atau fallback <Server /> -->
          <div class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0 shadow-sm group-hover:opacity-90 transition-opacity"
               :class="s.os_name ? (getDistroColorClass(s.os_name) || 'bg-slate-100 dark:bg-slate-700') : 'bg-brand-100 dark:bg-brand-900/30'">
            <img v-if="getDistroIcon(s.os_name)" :src="getDistroIcon(s.os_name)" :alt="s.os_name" class="w-7 h-7 object-contain" />
            <Server v-else class="w-6 h-6 text-brand-600 dark:text-brand-400 group-hover:text-brand-700" />
          </div>
          <div class="flex-1 min-w-0">
            <h3 class="font-bold text-slate-800 dark:text-slate-100 text-lg leading-tight truncate" :title="s.name">
              {{ s.name }}
            </h3>
            <div class="text-xs text-slate-500 dark:text-slate-400 font-mono mt-1 truncate">{{ s.url }}</div>
            <div v-if="s.os_name" class="text-[10px] text-slate-400 dark:text-slate-500 mt-0.5 truncate">{{ s.os_name }}</div>
          </div>
        </RouterLink>

        <!-- Footer Menu (Quick Links) -->
        <div class="p-2 border-t border-slate-100 dark:border-slate-700 bg-white dark:bg-slate-800 grid grid-cols-6 divide-x divide-slate-100 dark:divide-slate-700">
          <RouterLink :to="`/server/${s.id}/speedtest`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-400 hover:bg-slate-50 dark:hover:bg-slate-700 rounded transition-colors" title="Speedtest">
            <Activity class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/ports`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-400 hover:bg-slate-50 dark:hover:bg-slate-700 rounded transition-colors" title="Ports & Scan">
            <ShieldCheck class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/containers`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-400 hover:bg-slate-50 dark:hover:bg-slate-700 rounded transition-colors" title="Containers">
            <Box class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/cloudflare`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-400 hover:bg-slate-50 dark:hover:bg-slate-700 rounded transition-colors" title="Cloudflare">
            <Cloud class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/files`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-400 hover:bg-slate-50 dark:hover:bg-slate-700 rounded transition-colors" title="File Explorer">
            <FolderTree class="w-4 h-4" />
          </RouterLink>
          <RouterLink :to="`/server/${s.id}/settings`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-400 hover:bg-slate-50 dark:hover:bg-slate-700 rounded transition-colors" title="Settings">
            <Settings class="w-4 h-4" />
          </RouterLink>
        </div>

      </div>
    </div>
  </div>
</template>