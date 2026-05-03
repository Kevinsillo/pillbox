import { ref, onMounted, onUnmounted, type Ref } from 'vue'

export interface UsePollReturn {
    stop: () => void
    restart: () => void
    loaded: Ref<boolean>
}

/**
 * Periodically invokes `fn` every `intervalMs` and exposes lifecycle flags:
 *   - `loaded`: becomes `true` after the first successful run (used for fade-in).
 *
 * The poll is started on mount and stopped on unmount automatically.
 * Use `restart()` to immediately fire a new run (e.g. after user input changes).
 */
export function usePoll(fn: () => Promise<void> | void, intervalMs: number): UsePollReturn {
    const loaded = ref(false)

    let timer: ReturnType<typeof setTimeout> | null = null
    let stopped = false
    let inFlight = false

    function clearTimer() {
        if (timer !== null) {
            clearTimeout(timer)
            timer = null
        }
    }

    function schedule() {
        clearTimer()
        if (stopped) return
        timer = setTimeout(tick, intervalMs)
    }

    async function tick() {
        if (stopped || inFlight) {
            schedule()
            return
        }
        inFlight = true
        try {
            await fn()
            if (!loaded.value) {
                loaded.value = true
            }
        } catch {
            // Swallow errors so polling continues; callers handle their own error state.
        } finally {
            inFlight = false
            schedule()
        }
    }

    function stop() {
        stopped = true
        clearTimer()
    }

    function restart() {
        stopped = false
        clearTimer()
        tick()
    }

    onMounted(() => {
        stopped = false
        tick()
    })

    onUnmounted(() => {
        stop()
    })

    return { stop, restart, loaded }
}
