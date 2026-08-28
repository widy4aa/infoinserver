<script setup>
import { computed } from 'vue'
import { useToastStore } from '../stores/toastStore'
import { AlertCircle, CheckCircle2, Info, AlertTriangle, X } from 'lucide-vue-next'

const { state, closeToast } = useToastStore()

const iconComponent = computed(() => {
  switch (state.value.type) {
    case 'success': return CheckCircle2
    case 'error': return AlertCircle
    case 'warning': return AlertTriangle
    case 'confirm': return AlertTriangle
    default: return Info
  }
})

const iconColor = computed(() => {
  switch (state.value.type) {
    case 'success': return 'text-green-500'
    case 'error': return 'text-red-500'
    case 'warning': return 'text-amber-500'
    case 'confirm': return 'text-amber-500'
    default: return 'text-brand-500'
  }
})

const bgClass = computed(() => {
  return 'bg-white border border-slate-200'
})
</script>

<template>
  <div v-if="state.isOpen && state.type === 'confirm'" class="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-[100] flex items-center justify-center p-4">
    <div class="bg-white rounded-xl shadow-xl w-full max-w-md overflow-hidden animate-in fade-in zoom-in-95 duration-200">
      <div class="p-5 flex gap-4">
        <div class="shrink-0 mt-0.5">
          <component :is="iconComponent" class="w-6 h-6" :class="iconColor" />
        </div>
        <div class="flex-1">
          <h3 class="text-lg font-bold text-slate-800">{{ state.title }}</h3>
          <p class="text-slate-600 mt-2 text-sm leading-relaxed whitespace-pre-wrap">{{ state.message }}</p>
        </div>
      </div>
      <div class="bg-slate-50 px-5 py-4 border-t border-slate-100 flex justify-end gap-2">
        <button @click="state.onCancel" class="btn-outline">Cancel</button>
        <button @click="state.onConfirm" class="btn-primary" :class="state.title.includes('DANGER') ? '!bg-red-600 hover:!bg-red-700' : ''">Confirm</button>
      </div>
    </div>
  </div>

  <div v-else-if="state.isOpen" class="fixed bottom-4 right-4 z-[100] max-w-sm w-full animate-in slide-in-from-bottom-4 fade-in duration-300">
    <div class="rounded-lg shadow-lg p-4 flex gap-3 items-start" :class="bgClass">
      <component :is="iconComponent" class="w-5 h-5 shrink-0 mt-0.5" :class="iconColor" />
      <div class="flex-1 min-w-0">
        <h4 class="font-semibold text-sm text-slate-800">{{ state.title }}</h4>
        <p class="text-sm text-slate-600 mt-1 break-words leading-relaxed whitespace-pre-wrap">{{ state.message }}</p>
      </div>
      <button @click="closeToast" class="shrink-0 text-slate-400 hover:text-slate-600 p-1 -mr-2 -mt-2">
        <X class="w-4 h-4" />
      </button>
    </div>
  </div>
</template>
