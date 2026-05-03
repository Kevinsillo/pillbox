import { ref, watch, onUnmounted, type Ref } from 'vue'

/**
 * Returns a ref that animates towards `target` whenever the source ref changes.
 *
 * - Uses `requestAnimationFrame` with an ease-out cubic curve.
 * - Cancels any in-flight animation on each new target so values don't lerp twice.
 * - Initial value is the first observed `target` (no animation on mount).
 */
export function useTween(target: Ref<number>, durationMs: number): Ref<number> {
    const current = ref<number>(target.value ?? 0)
    let rafId: number | null = null

    function cancel() {
        if (rafId !== null) {
            cancelAnimationFrame(rafId)
            rafId = null
        }
    }

    function easeOutCubic(t: number): number {
        return 1 - Math.pow(1 - t, 3)
    }

    watch(target, (next, prev) => {
        cancel()
        if (typeof next !== 'number' || Number.isNaN(next)) return
        const from = typeof prev === 'number' && !Number.isNaN(prev) ? current.value : next
        // Ensure starting point matches what's currently rendered.
        current.value = from
        if (next === from || durationMs <= 0) {
            current.value = next
            return
        }
        const start = performance.now()
        const delta = next - from

        function step(now: number) {
            const elapsed = now - start
            const t = Math.min(1, elapsed / durationMs)
            const eased = easeOutCubic(t)
            current.value = from + delta * eased
            if (t < 1) {
                rafId = requestAnimationFrame(step)
            } else {
                rafId = null
                current.value = next
            }
        }

        rafId = requestAnimationFrame(step)
    })

    onUnmounted(cancel)

    return current
}
