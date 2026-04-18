import { ref, watch } from 'vue'
import type { Ref } from 'vue'

export function useLocalStorage<T>(key: string, defaultValue: T): Ref<T> {
    const stored = localStorage.getItem(key)
    const value = ref<T>(stored !== null ? (JSON.parse(stored) as T) : defaultValue) as Ref<T>

    watch(value, (v) => {
        if (v === null || v === undefined) {
            localStorage.removeItem(key)
        } else {
            localStorage.setItem(key, JSON.stringify(v))
        }
    }, { deep: true })

    return value
}
