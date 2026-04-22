import { watch } from 'vue'
import { useLocalStorage } from './useLocalStorage'

export type Theme = 'light' | 'dark'

export function useTheme() {
    const theme = useLocalStorage<Theme>('pillbox:theme', 'dark')

    const applyTheme = (t: Theme) => {
        document.documentElement.classList.toggle('dark', t === 'dark')
    }

    applyTheme(theme.value)
    watch(theme, applyTheme)

    const toggleTheme = () => {
        theme.value = theme.value === 'dark' ? 'light' : 'dark'
    }

    return { theme, toggleTheme }
}
