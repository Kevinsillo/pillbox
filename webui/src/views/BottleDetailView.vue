<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfirm } from '@/composables/useConfirm'
import { bottlesApi } from '@/core/infrastructure/repositories/BottlesRepository'
import type { Bottle, Prescription } from '@/core/domain/types'
import { RouterLink, useRouter } from 'vue-router'
import IArrowLeft from '~icons/lucide/arrow-left'
import IClipboard from '~icons/lucide/clipboard'
import ITrash2 from '~icons/lucide/trash-2'
import IZap from '~icons/lucide/zap'
import PrescriptionStatusBadge from '@/components/PrescriptionStatusBadge.vue'
import { useActiveBottle } from '@/composables/useActiveBottle'

const { t } = useI18n()
const { prompt, alert } = useConfirm()
const router = useRouter()
const props = defineProps<{ bottle_id: string }>()
const { activeBottleId } = useActiveBottle()

const bottle = ref<Bottle | null>(null)
const prescriptions = ref<Prescription[]>([])
const loading = ref(false)

async function load() {
    loading.value = true
    try {
        const [b, rx] = await Promise.all([
            bottlesApi.get(props.bottle_id),
            bottlesApi.prescriptions(props.bottle_id),
        ])
        bottle.value = b
        prescriptions.value = rx
    } finally {
        loading.value = false
    }
}

onMounted(load)

const isOpen = (rx: Prescription) => rx.ended_at === null && rx.deleted_at === null

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
        router.push('/bottles')
    } catch (e: unknown) {
        await alert(e instanceof Error ? e.message : t('common.error'))
    }
}

</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <RouterLink to="/bottles" class="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-300 transition-colors"><IArrowLeft class="w-3 h-3" /> {{ $t('bottle_detail.back') }}</RouterLink>

        <div v-if="loading" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>

        <template v-else-if="bottle">
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
                        class="flex items-center gap-1.5 text-sm text-red-400 hover:text-red-300 border border-red-900/40 px-3 py-2 rounded-lg transition-colors"
                    >
                        <ITrash2 class="w-3.5 h-3.5" />
                        {{ $t('common.delete') }}
                    </button>
                </div>
            </div>

            <div>
                <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">
                    {{ $t('bottle_detail.prescriptions_heading') }} ({{ prescriptions.length }})
                </h2>
                <div v-if="prescriptions.length === 0" class="text-zinc-500 text-sm">{{ $t('bottle_detail.empty') }}</div>
                <div v-else class="space-y-2">
                    <RouterLink
                        v-for="rx in prescriptions"
                        :key="rx.id"
                        :to="`/bottles/${props.bottle_id}/prescriptions/${rx.id}`"
                        class="flex items-center justify-between bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
                    >
                        <div class="flex items-center gap-3 min-w-0">
                            <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                                <IClipboard class="w-4 h-4 text-zinc-400" />
                            </div>
                            <div class="space-y-1 min-w-0">
                                <div class="flex items-center gap-2">
                                    <PrescriptionStatusBadge :open="isOpen(rx)" />
                                    <span class="text-(--text-h) text-sm font-medium">{{ rx.title }}</span>
                                </div>
                                <p class="text-xs text-zinc-600">{{ new Date(rx.started_at).toLocaleString() }}</p>
                            </div>
                        </div>
                    </RouterLink>
                </div>
            </div>
        </template>
    </div>
</template>
