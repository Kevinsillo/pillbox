<script setup lang="ts">
import { ref, computed, watch, toRef } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useConfirm } from '@/composables/useConfirm'
import { usePoll } from '@/composables/usePoll'
import { usePaginatedList } from '@/composables/usePaginatedList'
import { prescriptionsApi } from '@/core/infrastructure/repositories/PrescriptionsRepository'
import { ApiError } from '@/core/infrastructure/managers/httpClient'
import { shortId } from '@/core/utils/id'
import type { Prescription, Pill } from '@/core/domain/types'
import { formatAuthor } from '@/core/domain/author'
import PillCard from '@/components/PillCard.vue'
import PrescriptionStatusBadge from '@/components/PrescriptionStatusBadge.vue'
import Paginator from '@/components/Paginator.vue'
import CopyableId from '@/components/CopyableId.vue'
import HeaderMenu from '@/components/HeaderMenu.vue'
import ViewsBadge from '@/components/ViewsBadge.vue'
import IArrowLeft from '~icons/lucide/arrow-left'
import IClipboard from '~icons/lucide/clipboard'
import ILock from '~icons/lucide/lock'
import ITrash2 from '~icons/lucide/trash-2'

const { t } = useI18n()
const { confirm } = useConfirm()
const props = defineProps<{ bottle_id: string; rx_id: string }>()
const router = useRouter()

const rx = ref<Prescription | null>(null)

const bottleIdRef = toRef(props, 'bottle_id')
const rxIdRef = toRef(props, 'rx_id')

const {
    items: pills,
    total,
    page,
    pageSize,
    refresh: refreshPills,
} = usePaginatedList<Pill>({
    fetcher: (p) => prescriptionsApi.pills(bottleIdRef.value, rxIdRef.value, p),
    resetOn: [bottleIdRef, rxIdRef],
})

const sortedPills = computed(() =>
    [...pills.value].sort((a, b) => b.created_at.localeCompare(a.created_at))
)

let currentToken = 0

async function load() {
    const token = ++currentToken
    const [r] = await Promise.all([
        prescriptionsApi.get(props.bottle_id, props.rx_id),
        refreshPills(),
    ])
    if (token !== currentToken) return
    rx.value = r
}

const poll = usePoll(load, 5000)

watch(
    () => [props.bottle_id, props.rx_id] as const,
    () => {
        rx.value = null
        poll.restart()
    },
)

const isOpen = computed(() => rx.value?.ended_at === null && rx.value?.deleted_at === null)
const isArchived = computed(() => !!rx.value?.deleted_at)
const isClosed = computed(() => !!rx.value?.ended_at && !rx.value?.deleted_at)
const reopenError = ref<string | null>(null)
const authorDisplay = computed(() =>
    rx.value ? formatAuthor(rx.value.author_name, rx.value.author_email) : null
)

async function closeRx() {
    try {
        await confirm(
            t('confirm.close_prescription_msg'),
            t('confirm.close_prescription_title'),
            { confirmText: t('prescription_detail.close_btn'), cancelText: t('common.cancel'), waitSeconds: 3 }
        )
        const updated = await prescriptionsApi.close(props.bottle_id, props.rx_id)
        rx.value = updated
    } catch { /* cancelled */ }
}

async function reopenRx() {
    reopenError.value = null
    try {
        await confirm(
            t('confirm.reopen_prescription_msg'),
            t('confirm.reopen_prescription_title'),
            { confirmText: t('prescription_detail.reopen_btn'), cancelText: t('common.cancel'), waitSeconds: 3 }
        )
    } catch {
        return /* cancelled */
    }
    try {
        const updated = await prescriptionsApi.reopen(props.bottle_id, props.rx_id)
        rx.value = updated
        poll.restart()
    } catch (e) {
        if (e instanceof ApiError && e.code === 'prescription_collision') {
            const data = e.data as { existing_id?: string } | null
            const existing = data?.existing_id ? shortId(data.existing_id) : ''
            reopenError.value = t('errors.prescription_collision', { existing_id: existing })
        } else if (e instanceof ApiError) {
            reopenError.value = e.message
        } else {
            reopenError.value = t('common.error')
        }
    }
}

async function archiveRx() {
    try {
        await confirm(
            t('confirm.delete_prescription_msg', { title: rx.value?.title }),
            t('confirm.delete_prescription_title'),
            { confirmText: t('common.archive'), cancelText: t('common.cancel') }
        )
        await prescriptionsApi.delete(props.bottle_id, props.rx_id)
        poll.restart()
    } catch { /* cancelled */ }
}

async function purgeRx() {
    try {
        await confirm(
            t('confirm.purge_prescription_msg', { title: rx.value?.title }),
            t('confirm.purge_prescription_title'),
            { confirmText: t('common.delete_permanent'), cancelText: t('common.cancel'), waitSeconds: 3 }
        )
        await prescriptionsApi.purge(props.bottle_id, props.rx_id)
        router.back()
    } catch { /* cancelled */ }
}
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <button class="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-300" @click="router.back()">
            <IArrowLeft class="w-3 h-3" /> {{ $t('common.back') }}
        </button>

        <div v-if="!poll.loaded.value" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>

        <template v-else-if="rx">
            <div
                class="space-y-5 transition-opacity duration-300"
                :class="poll.loaded.value ? 'opacity-100' : 'opacity-0'"
            >
                <!-- Header -->
                <div class="space-y-3">
                    <div class="flex items-start justify-between gap-3">
                        <div class="flex gap-3 min-w-0">
                            <div class="min-h-full w-20 rounded-lg bg-(--bg-surface) border border-(--border) flex items-center justify-center shrink-0">
                                <IClipboard class="size-8 text-zinc-400" />
                            </div>
                            <div class="min-w-0">
                                <div class="mb-1 flex items-center gap-2 flex-wrap">
                                    <PrescriptionStatusBadge :open="isOpen" />
                                    <span v-if="isArchived" class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-(--badge-zinc-bg) text-(--badge-zinc-text)">{{ $t('prescription_detail.archived_badge') }}</span>
                                </div>
                                <div class="flex items-start gap-2">
                                    <CopyableId :id="rx.id" />
                                    <h1 class="text-xl font-bold text-(--text-h)">{{ rx.title }}</h1>
                                </div>
                                <p class="text-xs text-zinc-500 mt-0.5 flex items-center gap-2 flex-wrap">
                                    <ViewsBadge :views="rx.views" />
                                    <span>
                                        {{ $t('prescription_detail.started_at') }} {{ new Date(rx.started_at).toLocaleString() }}
                                        <span v-if="rx.ended_at"> · {{ $t('prescription_detail.closed_at') }} {{ new Date(rx.ended_at).toLocaleString() }}</span>
                                    </span>
                                </p>
                                <p v-if="authorDisplay" class="text-xs text-zinc-600 mt-0.5">{{ authorDisplay }}</p>
                            </div>
                        </div>
                        <HeaderMenu entity="prescription" :bottle-id="props.bottle_id" :rx-id="props.rx_id" :rx-closed="isClosed" />
                    </div>
                    <div class="flex gap-2">
                        <button v-if="isOpen"
                                class="text-sm text-zinc-400 hover:text-(--text-h) border border-(--border) px-3 py-2 rounded-lg"
                                @click="closeRx">
                            {{ $t('prescription_detail.close_btn') }}
                        </button>
                        <button v-if="isClosed"
                                class="text-sm text-emerald-300 hover:text-emerald-200 border border-emerald-900/40 px-3 py-2 rounded-lg"
                                @click="reopenRx">
                            {{ $t('prescription_detail.reopen_btn') }}
                        </button>
                        <button
                            v-if="!isArchived"
                            class="flex items-center gap-1.5 text-sm text-red-400 hover:text-red-300 border border-red-900/40 px-3 py-2 rounded-lg"
                            @click="archiveRx">
                            <ITrash2 class="w-3.5 h-3.5" />
                            {{ $t('common.archive') }}
                        </button>
                        <button
                            v-if="isArchived"
                            class="flex items-center gap-1.5 text-sm text-red-400 hover:text-red-300 border border-red-900/40 px-3 py-2 rounded-lg"
                            @click="purgeRx">
                            <ITrash2 class="w-3.5 h-3.5" />
                            {{ $t('common.delete_permanent') }}
                        </button>
                    </div>
                    <div v-if="reopenError" class="text-sm text-red-300 border border-red-900/40 bg-red-950/30 px-3 py-2 rounded-lg">
                        {{ reopenError }}
                    </div>
                    <div v-if="isClosed" class="flex items-start gap-2 text-sm text-(--callout-warn-text) border border-(--callout-warn-border) bg-(--callout-warn-bg) px-3 py-2 rounded-lg">
                        <ILock class="w-4 h-4 mt-0.5 shrink-0" />
                        <span>{{ $t('prescription_detail.closed_hint') }}</span>
                    </div>
                </div>

                <!-- Pills -->
                <div>
                    <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">
                        {{ $t('prescription_detail.pills_heading') }} ({{ total }})
                    </h2>
                    <div v-if="pills.length === 0" class="text-zinc-500 text-sm">{{ $t('prescription_detail.empty') }}</div>
                    <template v-else>
                        <TransitionGroup name="list" tag="div" class="space-y-3 relative">
                            <div
                                v-for="pill in sortedPills"
                                :key="pill.id"
                                :class="{ 'opacity-50 relative': pill.deleted_at }"
                            >
                                <PillCard :pill="pill" :bottle-id="props.bottle_id" />
                                <span
                                    v-if="pill.deleted_at"
                                    class="absolute top-2 right-2 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-(--badge-zinc-bg) text-(--badge-zinc-text) pointer-events-none"
                                >{{ $t('prescription_detail.archived_badge') }}</span>
                            </div>
                        </TransitionGroup>

                        <div class="pt-4 flex justify-center">
                            <Paginator v-model:current-page="page" :total="total" :page-size="pageSize" />
                        </div>
                    </template>
                </div>
            </div>
        </template>
    </div>
</template>
