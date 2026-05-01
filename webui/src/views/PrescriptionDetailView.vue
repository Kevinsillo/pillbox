<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useConfirm } from '@/composables/useConfirm'
import { prescriptionsApi } from '@/core/infrastructure/repositories/PrescriptionsRepository'
import type { Prescription, Pill } from '@/core/domain/types'
import { formatAuthor } from '@/core/domain/author'
import PillCard from '@/components/PillCard.vue'
import PrescriptionStatusBadge from '@/components/PrescriptionStatusBadge.vue'
import IArrowLeft from '~icons/lucide/arrow-left'
import IClipboard from '~icons/lucide/clipboard'
import ITrash2 from '~icons/lucide/trash-2'

const { t } = useI18n()
const { confirm } = useConfirm()
const props = defineProps<{ bottle_id: string; rx_id: string }>()
const router = useRouter()

const rx = ref<Prescription | null>(null)
const pills = ref<Pill[]>([])
const loading = ref(false)

async function load() {
    loading.value = true
    try {
        const [r, p] = await Promise.all([
            prescriptionsApi.get(props.bottle_id, props.rx_id),
            prescriptionsApi.pills(props.bottle_id, props.rx_id),
        ])
        rx.value = r
        pills.value = p.sort((a, b) => b.created_at.localeCompare(a.created_at))
    } finally {
        loading.value = false
    }
}

onMounted(load)

const isOpen = computed(() => rx.value?.ended_at === null && rx.value?.deleted_at === null)
const isArchived = computed(() => !!rx.value?.deleted_at)
const authorDisplay = computed(() =>
    rx.value ? formatAuthor(rx.value.author_name, rx.value.author_email) : null
)

const activePills = computed(() => pills.value.filter(p => p.deleted_at === null))
const archivedPills = computed(() => pills.value.filter(p => p.deleted_at !== null))

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

async function archiveRx() {
    try {
        await confirm(
            t('confirm.delete_prescription_msg', { title: rx.value?.title }),
            t('confirm.delete_prescription_title'),
            { confirmText: t('common.archive'), cancelText: t('common.cancel') }
        )
        await prescriptionsApi.delete(props.bottle_id, props.rx_id)
        await load()
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
        <button class="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-300 transition-colors" @click="router.back()">
            <IArrowLeft class="w-3 h-3" /> {{ $t('common.back') }}
        </button>

        <div v-if="loading" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>

        <template v-else-if="rx">
            <!-- Header -->
            <div class="space-y-3">
                <div class="flex gap-3">
                    <div class="min-h-full w-20 rounded-lg bg-(--bg-surface) border border-(--border) flex items-center justify-center shrink-0">
                        <IClipboard class="size-8 text-zinc-400" />
                    </div>
                    <div>
                        <div class="mb-1 flex items-center gap-2 flex-wrap">
                            <PrescriptionStatusBadge :open="isOpen" />
                            <span v-if="isArchived" class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-(--badge-zinc-bg) text-(--badge-zinc-text)">{{ $t('prescription_detail.archived_badge') }}</span>
                        </div>
                        <h1 class="text-xl font-bold text-(--text-h)">{{ rx.title }}</h1>
                        <p class="text-xs text-zinc-500 mt-0.5">
                            {{ $t('prescription_detail.started_at') }} {{ new Date(rx.started_at).toLocaleString() }}
                            <span v-if="rx.ended_at"> · {{ $t('prescription_detail.closed_at') }} {{ new Date(rx.ended_at).toLocaleString() }}</span>
                        </p>
                        <p v-if="authorDisplay" class="text-xs text-zinc-600 mt-0.5">{{ authorDisplay }}</p>
                    </div>
                </div>
                <div class="flex gap-2">
                    <button v-if="isOpen"
                            class="text-sm text-zinc-400 hover:text-(--text-h) border border-(--border) px-3 py-2 rounded-lg transition-colors"
                            @click="closeRx">
                        {{ $t('prescription_detail.close_btn') }}
                    </button>
                    <button
                        v-if="!isArchived"
                        class="flex items-center gap-1.5 text-sm text-red-400 hover:text-red-300 border border-red-900/40 px-3 py-2 rounded-lg transition-colors"
                        @click="archiveRx">
                        <ITrash2 class="w-3.5 h-3.5" />
                        {{ $t('common.archive') }}
                    </button>
                    <button
                        v-if="isArchived"
                        class="flex items-center gap-1.5 text-sm text-red-400 hover:text-red-300 border border-red-900/40 px-3 py-2 rounded-lg transition-colors"
                        @click="purgeRx">
                        <ITrash2 class="w-3.5 h-3.5" />
                        {{ $t('common.delete_permanent') }}
                    </button>
                </div>
            </div>

            <!-- Pills -->
            <div>
                <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">
                    {{ $t('prescription_detail.pills_heading') }} ({{ activePills.length }})
                </h2>
                <div v-if="pills.length === 0" class="text-zinc-500 text-sm">{{ $t('prescription_detail.empty') }}</div>
                <div v-else class="space-y-3">
                    <PillCard
                        v-for="pill in activePills"
                        :key="pill.id"
                        :pill="pill"
                        :bottle-id="props.bottle_id"
                    />

                    <template v-if="archivedPills.length > 0">
                        <h3 class="text-xs font-semibold text-zinc-600 uppercase tracking-wider mt-4 mb-2">
                            {{ $t('prescription_detail.archived_pills_heading') }} ({{ archivedPills.length }})
                        </h3>
                        <div v-for="pill in archivedPills" :key="pill.id" class="opacity-50 relative">
                            <PillCard
                                :pill="pill"
                                :bottle-id="props.bottle_id"
                            />
                            <span class="absolute top-2 right-2 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-(--badge-zinc-bg) text-(--badge-zinc-text) pointer-events-none">{{ $t('prescription_detail.archived_badge') }}</span>
                        </div>
                    </template>
                </div>
            </div>
        </template>
    </div>
</template>
