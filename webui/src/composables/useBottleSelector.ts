import { computed, onMounted, ref } from "vue"
import { useRouter } from "vue-router"
import { useActiveBottle } from "@/composables/useActiveBottle"
import type { Bottle } from "@/core/domain/types"
import { bottlesApi } from "@/core/infrastructure/repositories/BottlesRepository"

/**
 * Encapsulates the bottles list fetch + auto-select logic used by MainLayout.
 * - Loads up to 100 bottles on mount.
 * - Filters to linked bottles for selection.
 * - Clears the active bottle id from localStorage if it no longer exists among linked ones.
 * - If no active bottle is set and at least one linked bottle exists, auto-selects the first.
 * - Exposes a `switchBottle` action that routes to "/" (used as the @change handler).
 */
export function useBottleSelector() {
    const router = useRouter()
    const { activeBottleId } = useActiveBottle()

    const bottles = ref<Bottle[]>([])
    const linkedBottles = computed(() => bottles.value.filter(b => b.linked))

    onMounted(async () => {
        try {
            const result = await bottlesApi.list({ page: 1, page_size: 100 })
            bottles.value = result.items
            // Si el bottle activo ya no existe entre los vinculados, limpiar localStorage
            if (activeBottleId.value !== null && !linkedBottles.value.find(b => b.id === activeBottleId.value)) {
                activeBottleId.value = null
            }
            if (activeBottleId.value === null && linkedBottles.value.length > 0) {
                activeBottleId.value = linkedBottles.value[0].id
            }
        } catch {}
    })

    const switchBottle = () => {
        router.push("/")
    }

    return {
        bottles,
        linkedBottles,
        activeBottleId,
        switchBottle,
    }
}
