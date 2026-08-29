<script setup>
import { RouterView, RouterLink } from 'vue-router'
import { Server, RefreshCw } from 'lucide-vue-next'
import ToastAlert from './components/ToastAlert.vue'
import { useToastStore } from './stores/toastStore'

const { showConfirm, showToast } = useToastStore()

const handleUpdate = async () => {
  showConfirm(
    "Update Dashboard", 
    "Tindakan ini akan melakukan 'git pull' dan mengompilasi ulang dashboard secara remote. Lanjutkan?",
    async () => {
      try {
        const res = await fetch(`/api/system/update`, {method: 'POST'})
        const data = await res.json()
        if (res.ok) {
          showToast("Success", data.message, "success")
          setTimeout(() => window.location.reload(), 5000)
        } else {
          showToast("Error", data, "error")
        }
      } catch(e) {
        showToast("Error", e.message, "error")
      }
    }
  )
}
</script>

<template>
  <div class="min-h-screen flex flex-col bg-slate-100 dark:bg-slate-950">
    <!-- Top Navigation -->
    <header class="bg-white border-b border-slate-200 sticky top-0 z-40 dark:bg-slate-900 dark:border-slate-800">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="flex justify-between items-center h-16">
          <RouterLink to="/" class="flex items-center gap-2 hover:opacity-80 transition-opacity">
            <Server class="w-6 h-6 text-brand-600" />
            <h1 class="text-xl font-bold text-slate-800 dark:text-slate-100 tracking-tight hidden sm:block">infoinserver</h1>
          </RouterLink>
          
          <div class="flex items-center gap-3">
            <button @click="handleUpdate" class="btn-outline text-brand-600" title="Pull changes and rebuild">
              <RefreshCw class="w-4 h-4" />
              <span class="hidden sm:inline">Update Dashboard</span>
            </button>
          </div>
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <main class="flex-1 w-full max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <RouterView />
    </main>

    <!-- Global Toast & Confirm Dialog -->
    <ToastAlert />
  </div>
</template>

<style>
/* Tab navigation styles with dark mode support */
.tab-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  white-space: nowrap;
  padding: 0.5rem 0.5rem;
  border-bottom-width: 2px;
  font-weight: 500;
  font-size: 0.875rem;
  transition-property: color, background-color, border-color;
  transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
  transition-duration: 150ms;
  border-color: transparent;
  color: #64748b;
  margin-bottom: -1px;
}

.dark .tab-item {
  color: #94a3b8;
}

.tab-item:hover {
  color: #334155;
  border-color: #cbd5e1;
}

.dark .tab-item:hover {
  color: #e2e8f0;
  border-color: #475569;
}

.tab-active {
  border-color: #3b82f6 !important;
  color: #2563eb !important;
}

.dark .tab-active {
  border-color: #60a5fa !important;
  color: #93c5fd !important;
}
</style>