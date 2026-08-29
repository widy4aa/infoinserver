import { ref } from 'vue'

// Dark mode disabled — always light mode
export const isDark = ref(false)
export const toggleDark = () => {}

export const useThemeStore = () => ({
  isDark,
  toggleDark,
})