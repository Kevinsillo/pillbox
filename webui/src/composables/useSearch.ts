import { ref, watch } from "vue"
import { useActiveBottle } from "@/composables/useActiveBottle"
import type { CapsuleSearchResult, Compound, PillSearchResult } from "@/core/domain/types"
import { capsulesApi } from "@/core/infrastructure/repositories/CapsulesRepository"
import { pillsApi } from "@/core/infrastructure/repositories/PillsRepository"

export type SearchScope = "all" | "pills" | "capsules"

/**
 * Encapsulates the SearchView state + behavior:
 * - Debounced search with AbortController.
 * - Paginated state for pills + capsules collections.
 * - Compound list loading and selection.
 */
export function useSearch() {
    const { activeBottleId } = useActiveBottle()

    const query = ref("")
    const scope = ref<SearchScope>("all")
    const compound = ref<string>("")
    const compounds = ref<Compound[]>([])
    const pillItems = ref<PillSearchResult[]>([])
    const pillsTotal = ref(0)
    const capItems = ref<CapsuleSearchResult[]>([])
    const capsTotal = ref(0)
    const loading = ref(false)
    const searched = ref(false)
    const isFuzzy = ref(false)

    const pillsPage = ref(1)
    const pillsPageSize = ref(20)
    const capsPage = ref(1)
    const capsPageSize = ref(20)

    let debounce: ReturnType<typeof setTimeout>
    let currentController: AbortController | null = null

    async function loadCompounds() {
        try {
            if (scope.value === "all") {
                const [pillsC, capsC] = await Promise.all([
                    pillsApi.getCompounds(activeBottleId.value ?? undefined),
                    capsulesApi.getCompounds(),
                ])
                const map = new Map<string, number>()
                for (const c of pillsC) map.set(c.compound, (map.get(c.compound) ?? 0) + c.count)
                for (const c of capsC) map.set(c.compound, (map.get(c.compound) ?? 0) + c.count)
                compounds.value = Array.from(map.entries())
                    .map(([cmp, count]) => ({ compound: cmp, count }))
                    .sort((a, b) => b.count - a.count || a.compound.localeCompare(b.compound))
            } else if (scope.value === "pills") {
                compounds.value = await pillsApi.getCompounds(activeBottleId.value ?? undefined)
            } else {
                compounds.value = await capsulesApi.getCompounds()
            }
        } catch {
            compounds.value = []
        }
    }

    async function runSearch() {
        const q = query.value.trim()
        isFuzzy.value = false

        if (!q && !compound.value) {
            pillItems.value = []
            pillsTotal.value = 0
            capItems.value = []
            capsTotal.value = 0
            searched.value = false
            return
        }

        if (currentController) currentController.abort()
        currentController = new AbortController()

        loading.value = true
        searched.value = true

        const effectiveQuery = q
        const effectiveCompound = compound.value || undefined

        const runPills = () =>
            pillsApi.search({
                query: effectiveQuery,
                bottle_id: activeBottleId.value ?? undefined,
                compound: effectiveCompound,
                page: pillsPage.value,
                page_size: pillsPageSize.value,
            })
        const runCaps = () =>
            capsulesApi.search({
                query: effectiveQuery,
                compound: effectiveCompound,
                page: capsPage.value,
                page_size: capsPageSize.value,
            })

        try {
            // El backend decide internamente si reintenta con fuzzy y nos
            // devuelve `used_fuzzy` en la respuesta paginada.
            let pillsRes: { items: PillSearchResult[]; total: number; used_fuzzy?: boolean } = {
                items: [],
                total: 0,
            }
            let capsRes: { items: CapsuleSearchResult[]; total: number; used_fuzzy?: boolean } = {
                items: [],
                total: 0,
            }
            if (scope.value === "all") {
                ;[pillsRes, capsRes] = await Promise.all([runPills(), runCaps()])
            } else if (scope.value === "pills") {
                pillsRes = await runPills()
            } else {
                capsRes = await runCaps()
            }

            isFuzzy.value = Boolean(pillsRes.used_fuzzy || capsRes.used_fuzzy)
            pillItems.value = pillsRes.items
            pillsTotal.value = pillsRes.total
            capItems.value = capsRes.items
            capsTotal.value = capsRes.total
        } finally {
            loading.value = false
        }
    }

    function triggerSearch() {
        clearTimeout(debounce)
        debounce = setTimeout(runSearch, 150)
    }

    function resetPages() {
        pillsPage.value = 1
        capsPage.value = 1
    }

    function onInput() {
        isFuzzy.value = false
        resetPages()
        triggerSearch()
    }

    function onScopeChange() {
        isFuzzy.value = false
        compound.value = ""
        resetPages()
        loadCompounds()
        triggerSearch()
    }

    function onCompoundChange() {
        isFuzzy.value = false
        resetPages()
        triggerSearch()
    }

    watch(activeBottleId, () => {
        resetPages()
        loadCompounds()
        triggerSearch()
    })

    watch(pillsPage, () => {
        runSearch()
    })

    watch(capsPage, () => {
        runSearch()
    })

    return {
        // state
        query,
        scope,
        compound,
        compounds,
        pillItems,
        pillsTotal,
        capItems,
        capsTotal,
        loading,
        searched,
        isFuzzy,
        pillsPage,
        pillsPageSize,
        capsPage,
        capsPageSize,
        // actions
        runSearch,
        loadCompounds,
        onInput,
        onScopeChange,
        onCompoundChange,
    }
}
