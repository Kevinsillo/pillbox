<script setup lang="ts">
import { ref, computed, watch, toRef } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfirm } from '@/composables/useConfirm'
import { usePoll } from '@/composables/usePoll'
import { usePaginatedList } from '@/composables/usePaginatedList'
import { bottlesApi } from '@/core/infrastructure/repositories/BottlesRepository'
import type { Bottle, Prescription } from '@/core/domain/types'
import { RouterLink, useRouter } from 'vue-router'
import IArrowLeft from '~icons/lucide/arrow-left'
import ITrash2 from '~icons/lucide/trash-2'
import IZap from '~icons/lucide/zap'
import PrescriptionCard from '@/components/PrescriptionCard.vue'
import Paginator from '@/components/Paginator.vue'
import { useActiveBottle } from '@/composables/useActiveBottle'

const { t } = useI18n()
const { prompt, alert } = useConfirm()
const router = useRouter()
const props = defineProps<{ bottle_id: string }>()
const { activeBottleId } = useActiveBottle()

const bottle = ref<Bottle | null>(null)

const bottleIdRef = toRef(props, 'bottle_id')

const {
    items: prescriptions,
    total,
    page,
    pageSize,
    refresh: refreshPrescriptions,
} = usePaginatedList<Prescription>({
    fetcher: (p) => bottlesApi.prescriptions(bottleIdRef.value, p),
    resetOn: [bottleIdRef],
})

const activePrescriptions = computed(() => prescriptions.value.filter(rx => rx.deleted_at === null))
const archivedPrescriptions = computed(() => prescriptions.value.filter(rx => rx.deleted_at !== null))

let currentToken = 0

async function load() {
    const token = ++currentToken
    const [b] = await Promise.all([
        bottlesApi.get(props.bottle_id),
        refreshPrescriptions(),
    ])
    if (token !== currentToken) return
    bottle.value = b
}

const poll = usePoll(load, 5000)

watch(
    () => props.bottle_id,
    () => {
        bottle.value = null
        poll.restart()
    },
)

async function deleteBottle() {
    if (!bottle.value) return
    const slug = bottle.value.name
    let result
    try {
        result = await prompt(
            t('confirm.delete_bottle_msg', { name: slug }),
            t('confirm.delete_bottle_title'),
            {
                inputPlaceholder: slug,
                inputValidator: (v: string) => v === slug || t('confirm.delete_bottle_prompt'),
                confirmText: t('common.delete'),
                cancelText: t('common.cancel'),
            }
        )
    } catch { return }
    if (result.value !== slug) return
    try {
        await bottlesApi.deleteBottle(props.bottle_id)
        if (activeBottleId.value === props.bottle_id) activeBottleId.value = null
        router.push('/bottles')
    } catch (e: unknown) {
        await alert(e instanceof Error ? e.message : t('common.error'))
    }
}

</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <RouterLink to="/bottles" class="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-300"><IArrowLeft class="w-3 h-3" /> {{ $t('bottle_detail.back') }}</RouterLink>

        <div v-if="!poll.loaded.value" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>

        <template v-else-if="bottle">
            <div
                class="space-y-5 transition-opacity duration-300"
                :class="poll.loaded.value ? 'opacity-100' : 'opacity-0'"
            >
                <div class="space-y-3">
                    <div>
                        <div class="flex items-center gap-2">
                            <span v-if="bottle.id === activeBottleId"
                                  class="inline-flex items-center justify-center p-1 rounded-md bg-green-500/10 border border-green-500/20">
                                <IZap class="w-3 h-3 text-green-500" />
                            </span>
                            <h1 class="text-2xl font-bold text-(--text-h)">{{ bottle.display_name }}</h1>
                            <span class="text-sm text-zinc-600 font-mono">{{ bottle.name }}</span>
                        </div>
                        <div class="flex items-center gap-1.5 mt-0.5">
                            <span class="text-[11px] text-zinc-600 border border-(--border) px-1.5 py-0 rounded shrink-0">{{ bottle.scope }}</span>
                            <span class="text-xs text-zinc-500 truncate">{{ bottle.directory }}</span>
                        </div>
                    </div>
                    <div class="flex gap-2">
                        <button
                            @click="deleteBottle"
                            class="flex items-center gap-1.5 text-sm text-red-400 hover:text-red-300 border border-red-900/40 px-3 py-2 rounded-lg"
                        >
                            <ITrash2 class="w-3.5 h-3.5" />
                            {{ $t('common.delete') }}
                        </button>
                    </div>
                </div>

                <div>
                    <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">
                        {{ $t('bottle_detail.prescriptions_heading') }} ({{ total }})
                    </h2>
                    <div v-if="prescriptions.length === 0" class="text-zinc-500 text-sm">{{ $t('bottle_detail.empty') }}</div>
                    <template v-else>
                        <TransitionGroup name="list" tag="div" class="space-y-2 relative">
                            <PrescriptionCard
                                v-for="rx in activePrescriptions"
                                :key="`active-${rx.id}`"
                                :prescription="rx"
                                :bottle-id="props.bottle_id"
                            />
                        </TransitionGroup>

                        <template v-if="archivedPrescriptions.length > 0">
                            <h3 class="text-xs font-semibold text-zinc-600 uppercase tracking-wider mt-4 mb-2">
                                {{ $t('bottle_detail.archived_heading') }} ({{ archivedPrescriptions.length }})
                            </h3>
                            <TransitionGroup name="list" tag="div" class="space-y-2 relative">
                                <PrescriptionCard
                                    v-for="rx in archivedPrescriptions"
                                    :key="`archived-${rx.id}`"
                                    :prescription="rx"
                                    :bottle-id="props.bottle_id"
                                    :archived="true"
                                />
                            </TransitionGroup>
                        </template>

                        <div class="pt-4 flex justify-center">
                            <Paginator v-model:current-page="page" :total="total" :page-size="pageSize" />
                        </div>
                    </template>
                </div>
            </div>
        </template>
    </div>
</template>
