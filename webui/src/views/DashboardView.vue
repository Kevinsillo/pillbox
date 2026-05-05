<script setup lang="ts">
import { ElAlert } from 'element-plus'
import PillCard from '@/components/PillCard.vue'
import PillsActivityChart from '@/components/PillsActivityChart.vue'
import { useActiveBottle } from '@/composables/useActiveBottle'
import { usePoll } from '@/composables/usePoll'
import { useTween } from '@/composables/useTween'
import type { Bottle, BottleStats, Context, Prescription } from '@/core/domain/types'
import { bottlesApi } from '@/core/infrastructure/repositories/BottlesRepository'
import { contextApi } from '@/core/infrastructure/repositories/ContextRepository'
import { ApiError } from '@/core/infrastructure/managers/httpClient'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'
import IArrowRight from '~icons/lucide/arrow-right'
import PrescriptionCard from '@/components/PrescriptionCard.vue'
import TruncatedTitle from '@/components/TruncatedTitle.vue'

const { t } = useI18n()
const { activeBottleId } = useActiveBottle()

type Period = '1d' | '1w' | '1m' | '1y'

const ctx = ref<Context | null>(null)
const bottle = ref<Bottle | null>(null)
const prescriptions = ref<Prescription[]>([])
const stats = ref<BottleStats | null>(null)
const error = ref<string | null>(null)

const activePeriod = ref<Period>('1m')
const periodDays = ref<number>(30)

let currentToken = 0

async function load() {
    if (!activeBottleId.value) return
    const id = activeBottleId.value
    const token = ++currentToken
    error.value = null
    try {
        const [c, b, rx, s] = await Promise.all([
            contextApi.get(id, { prescription_limit: 5, pill_limit: 8 }),
            bottlesApi.get(id),
            bottlesApi.prescriptions(id, 5),
            bottlesApi.stats(id, periodDays.value),
        ])
        if (token !== currentToken) return
        ctx.value = c
        bottle.value = b
        prescriptions.value = rx
        stats.value = s
    } catch (e: unknown) {
        if (token !== currentToken) return
        if (e instanceof ApiError && e.code === 'bottle_not_found') {
            activeBottleId.value = null
            return
        }
        error.value = e instanceof Error ? e.message : t('common.error')
    }
}

const poll = usePoll(load, 5000)

watch(activeBottleId, (id) => {
    if (id !== null) {
        // Reset state so loaded fade-in plays again on bottle switch.
        ctx.value = null
        bottle.value = null
        prescriptions.value = []
        stats.value = null
        poll.restart()
    } else {
        poll.stop()
    }
})

function onPeriodChange(days: number, key: Period) {
    activePeriod.value = key
    periodDays.value = days
    poll.restart()
}

const openRx = computed(() => prescriptions.value.find(rx => rx.ended_at === null))
const closedRx = computed(() => prescriptions.value.filter(rx => rx.ended_at !== null))

const pillCountTarget = computed(() => ctx.value?.pill_count ?? 0)
const rxCountTarget = computed(() => ctx.value?.prescription_count ?? 0)
const pillCountTween = useTween(pillCountTarget, 600)
const rxCountTween = useTween(rxCountTarget, 600)
const displayPillCount = computed(() => Math.round(pillCountTween.value))
const displayRxCount = computed(() => Math.round(rxCountTween.value))
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-6">
        <!-- Header -->
        <div>
            <h1 class="text-2xl font-bold text-(--text-h)">{{ $t('dashboard.heading') }}</h1>
            <p v-if="bottle" class="text-sm text-zinc-500 mt-0.5">{{ bottle.display_name }} · {{ bottle.directory }}</p>
        </div>

        <div v-if="!activeBottleId" class="text-center py-16 text-zinc-500">
            <p class="text-4xl mb-3">⬢</p>
            <p>{{ $t('dashboard.no_bottle_hint') }}</p>
            <RouterLink to="/bottles" class="text-sm text-zinc-400 hover:text-(--text-h) underline mt-2 inline-block">
                {{ $t('dashboard.manage_bottles') }} <IArrowRight class="w-3 h-3 inline" />
            </RouterLink>
        </div>

        <template v-else>
            <div v-if="!poll.loaded.value" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>
            <el-alert
                v-else-if="error"
                :title="$t('common.error_loading_title')"
                :description="error"
                type="error"
                show-icon
                :closable="false"
            />

            <template v-else-if="ctx">
                <div
                    class="space-y-6 transition-opacity duration-300"
                    :class="poll.loaded.value ? 'opacity-100' : 'opacity-0'"
                >
                    <!-- Stats + chart -->
                    <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
                        <div class="bg-(--bg-surface) border border-(--border) rounded-lg p-4 flex flex-col items-center justify-center text-center">
                            <p class="text-2xl font-bold text-(--text-h) tabular-nums">{{ displayPillCount }}</p>
                            <p class="text-xs text-zinc-500 mt-1">{{ $t('dashboard.stat_pills') }}</p>
                        </div>
                        <div class="bg-(--bg-surface) border border-(--border) rounded-lg p-4 flex flex-col items-center justify-center text-center">
                            <p class="text-2xl font-bold text-(--text-h) tabular-nums">{{ displayRxCount }}</p>
                            <p class="text-xs text-zinc-500 mt-1">{{ $t('dashboard.stat_prescriptions') }}</p>
                        </div>
                        <div class="sm:col-span-2">
                            <PillsActivityChart
                                :stats="stats"
                                :active-period="activePeriod"
                                @period-change="onPeriodChange"
                            />
                        </div>
                    </div>

                    <!-- Rx abierta -->
                    <RouterLink v-if="openRx" :to="`/bottles/${activeBottleId}/prescriptions/${openRx.id}`"
                                class="block bg-(--bg-surface) border border-green-900/40 rounded-lg p-4 hover:border-green-700/60 transition-colors">
                        <p class="text-xs text-green-500 uppercase tracking-wider mb-1">{{ $t('dashboard.open_rx_label') }}</p>
                        <TruncatedTitle tag="p" :title="openRx.title" class="text-(--text-h) font-medium" />
                        <p class="text-xs text-zinc-500 mt-0.5">{{ new Date(openRx.started_at).toLocaleString() }}</p>
                    </RouterLink>

                    <!-- Últimas Rx -->
                    <div v-if="closedRx.length > 0">
                        <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">{{ $t('dashboard.last_prescriptions') }}</h2>
                        <TransitionGroup name="list" tag="div" class="space-y-2 relative">
                            <PrescriptionCard
                                v-for="rx in closedRx"
                                :key="rx.id"
                                :prescription="rx"
                                :bottle-id="activeBottleId ?? ''"
                            />
                        </TransitionGroup>
                        <RouterLink :to="`/bottles/${activeBottleId}`"
                                    class="text-xs text-zinc-500 hover:text-zinc-300 mt-2 inline-block">
                            <span class="flex items-center gap-1">{{ $t('dashboard.view_all') }} <IArrowRight class="w-3 h-3" /></span>
                        </RouterLink>
                    </div>

                    <!-- Contexto activo -->
                    <div v-if="ctx.context.length > 0">
                        <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">{{ $t('dashboard.recent_context') }}</h2>
                        <TransitionGroup name="list" tag="div" class="space-y-3 relative">
                            <PillCard v-for="pill in ctx.context" :key="pill.id" :pill="pill" :bottle-id="activeBottleId ?? ''" />
                        </TransitionGroup>
                    </div>
                </div>
            </template>
        </template>
    </div>
</template>
