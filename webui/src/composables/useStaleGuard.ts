/**
 * Guards against stale async results from overlapping invocations.
 *
 * Each call to `next()` claims a fresh token and returns an `isCurrent()`
 * predicate. After awaiting, call `isCurrent()` to check whether a newer
 * invocation has started in the meantime — if so, the current result is
 * stale and should be discarded.
 *
 *   const guard = useStaleGuard()
 *   async function load() {
 *       const isCurrent = guard.next()
 *       const data = await fetchSomething()
 *       if (!isCurrent()) return   // a newer load() superseded this one
 *       state.value = data
 *   }
 */
export interface UseStaleGuardReturn {
    /** Claims a fresh token; returns a predicate that is `true` only while this remains the latest call. */
    next: () => () => boolean
}

export function useStaleGuard(): UseStaleGuardReturn {
    let currentToken = 0

    function next(): () => boolean {
        const token = ++currentToken
        return () => token === currentToken
    }

    return { next }
}
