import { ref } from 'vue'

// Dark mode disabled temporarily as requested
export const isDark = ref(false)

export const toggleDark = () => {
  // no-op while disabled
}

export const useThemeStore = () => ({
  isDark,
  toggleDark,
})