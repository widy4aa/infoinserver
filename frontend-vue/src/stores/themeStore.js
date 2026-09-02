import { ref } from 'vue'

const STORAGE_KEY = 'infoin-theme'

// Load saved preference dari localStorage, default: light mode
const saved = localStorage.getItem(STORAGE_KEY)
const isDark = ref(saved === 'dark')

// Apply class .dark ke <html> saat init agar Tailwind dark: variants aktif
if (isDark.value) {
  document.documentElement.classList.add('dark')
}

const toggleDark = () => {
  isDark.value = !isDark.value
  // Toggle class .dark di <html> — mengaktifkan seluruh Tailwind dark: prefix
  document.documentElement.classList.toggle('dark', isDark.value)
  // Persist ke localStorage
  localStorage.setItem(STORAGE_KEY, isDark.value ? 'dark' : 'light')
}

export const useThemeStore = () => ({ isDark, toggleDark })
