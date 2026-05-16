import { ref, watch, onMounted, type Ref, type WatchSource } from 'vue'
import type { Paginated, PaginationParams } from '@/core/domain/types'

export interface UsePaginatedListOptions<T> {
    /** Fetcher receives current pagination. Should return a Promise<Paginated<T>>. */
    fetcher: (params: PaginationParams) => Promise<Paginated<T>>
    /** Refs whose change triggers page=1 reset + refetch. Optional. */
    resetOn?: WatchSource[]
    /** Initial page_size. Default 20. */
    pageSize?: number
}

export interface UsePaginatedListReturn<T> {
    items: Ref<T[]>
    total: Ref<number>
    page: Ref<number>
    pageSize: Ref<number>
    loading: Ref<boolean>
    error: Ref<unknown>
    /** Manually reset to page 1 and refetch. */
    reset: () => Promise<void>
    /** Refetch current page (e.g. for polling). */
    refresh: () => Promise<void>
}

/**
 * Manages paginated state with auto-reset semantics.
 *
 *   - On mount: fetches page 1.
 *   - Setting `page` externally: refetches that page.
 *   - Changing `pageSize`: resets page to 1 and refetches.
 *   - Any change in `resetOn` sources: resets page to 1 and refetches
 *     (coalesces sync changes within the same microtask).
 *   - `reset()`: explicit page=1 + refetch.
 *   - `refresh()`: refetch current page without resetting (for polling).
 */
export function usePaginatedList<T>(options: UsePaginatedListOptions<T>): UsePaginatedListReturn<T> {
    const { fetcher, resetOn, pageSize: initialPageSize = 20 } = options

    const items = ref([]) as Ref<T[]>
    const total = ref(0)
    const page = ref(1)
    const pageSize = ref(initialPageSize)
    const loading = ref(false)
    const error = ref<unknown>(null)

    async function fetchCurrent(): Promise<void> {
        loading.value = true
        error.value = null
        try {
            const result = await fetcher({ page: page.value, page_size: pageSize.value })
            items.value = result.items
            total.value = result.total
        } catch (err) {
            error.value = err
        } finally {
            loading.value = false
        }
    }

    // Watch page changes: refetch on demand.
    watch(page, (newPage, oldPage) => {
        if (newPage === oldPage) return
        fetchCurrent()
    })

    // Watch pageSize: reset page to 1 and refetch. If page was already 1,
    // the page watcher won't fire, so call fetchCurrent manually.
    watch(pageSize, () => {
        if (page.value === 1) {
            fetchCurrent()
        } else {
            page.value = 1
        }
    })

    // Watch resetOn sources: coalesce multiple ref changes via microtask.
    if (resetOn && resetOn.length > 0) {
        let pending = false
        watch(resetOn, () => {
            if (pending) return
            pending = true
            queueMicrotask(() => {
                pending = false
                if (page.value === 1) {
                    fetchCurrent()
                } else {
                    page.value = 1
                }
            })
        })
    }

    async function reset(): Promise<void> {
        if (page.value === 1) {
            await fetchCurrent()
        } else {
            // Set page first then fetch; suppress the page watcher's fetch
            // by fetching manually after the reactive change.
            page.value = 1
            // The page watcher will fire and call fetchCurrent. We await
            // a microtask to let it dispatch, but to guarantee the caller
            // sees the result, we await our own fetch which is idempotent
            // for the current state.
            await fetchCurrent()
        }
    }

    async function refresh(): Promise<void> {
        await fetchCurrent()
    }

    onMounted(() => {
        fetchCurrent()
    })

    return {
        items,
        total,
        page,
        pageSize,
        loading,
        error,
        reset,
        refresh,
    }
}
