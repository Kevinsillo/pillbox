import { useLocalStorage } from './useLocalStorage'

const activeBottleId = useLocalStorage<number | null>('pillbox:active_bottle_id', null)

export function useActiveBottle() {
    return { activeBottleId }
}
