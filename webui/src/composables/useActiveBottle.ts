import { useLocalStorage } from './useLocalStorage'

// Migration: clear numeric IDs stored by old versions
const _raw = localStorage.getItem('pillbox:active_bottle_id')
if (_raw !== null && !isNaN(Number(_raw))) {
    localStorage.removeItem('pillbox:active_bottle_id')
}

const activeBottleId = useLocalStorage<string | null>('pillbox:active_bottle_id', null)

export function useActiveBottle() {
    return { activeBottleId }
}
