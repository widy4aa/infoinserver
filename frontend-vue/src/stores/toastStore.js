import { ref } from 'vue'

// Global reactive state menggunakan Vue Composition API (tanpa Pinia)
const state = ref({
  isOpen: false,
  title: '',
  message: '',
  type: 'info', // 'info', 'success', 'warning', 'error', 'confirm'
  onConfirm: null,
  onCancel: null,
})

export const useToastStore = () => {
  const showToast = (title, message, type = 'info') => {
    state.value = {
      isOpen: true,
      title,
      message,
      type,
      onConfirm: null,
      onCancel: null,
    }
    
    // Auto close unless it's an error
    if (type !== 'error') {
      setTimeout(() => {
        closeToast()
      }, 5000)
    }
  }

  const showConfirm = (title, message, onConfirmCallback, onCancelCallback = null) => {
    state.value = {
      isOpen: true,
      title,
      message,
      type: 'confirm',
      onConfirm: () => {
        closeToast()
        if (onConfirmCallback) onConfirmCallback()
      },
      onCancel: () => {
        closeToast()
        if (onCancelCallback) onCancelCallback()
      },
    }
  }

  const closeToast = () => {
    state.value.isOpen = false
  }

  return {
    state,
    showToast,
    showConfirm,
    closeToast
  }
}