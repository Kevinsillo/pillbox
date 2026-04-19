<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessageBox } from 'element-plus'
import { bottlesApi } from '@/core/infrastructure/repositories/BottlesRepository'
import { prescriptionsApi } from '@/core/infrastructure/repositories/PrescriptionsRepository'
import type { Bottle, Prescription } from '@/core/domain/types'
import { RouterLink } from 'vue-router'
import IArrowLeft from '~icons/lucide/arrow-left'
import IClipboard from '~icons/lucide/clipboard'

const { t } = useI18n()
const props = defineProps<{ id: string }>()

const bottle = ref<Bottle | null>(null)
const prescriptions = ref<Prescription[]>([])
const loading = ref(false)

async function load() {
    loading.value = true
    try {
        const [b, rx] = await Promise.all([
            bottlesApi.get(Number(props.id)),
            bottlesApi.prescriptions(Number(props.id)),
        ])
        bottle.value = b
        prescriptions.value = rx
    } finally {
        loading.value = false }
}

onMounted(load)

const isOpen = (rx: Prescription) => rx.ended_at === null && rx.deleted_at === null

async function deleteRx(rx: Prescription) {
    try {
        await ElMessageBox.confirm(
            t('confirm.delete_prescription_msg'),
            t('confirm.delete_prescription_title'),
            {
                confirmButtonText: t('common.delete'),
                cancelButtonText: t('common.cancel'),
                type: 'warning',
            }
        )
        await prescriptionsApi.delete(String(rx.id))
        prescriptions.value = prescriptions.value.filter(p => p.id !== rx.id)
    } catch { /* cancelled */ }
}
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <RouterLink to="/bottles" class="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-300 transition-colors"><IArrowLeft class="w-3 h-3" /> {{ $t('bottle_detail.back') }}</RouterLink>

        <div v-if="loading" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>

        <template v-else-if="bottle">
            <div class="flex items-start justify-between">
                <div>
                    <h1 class="text-2xl font-bold text-(--text-h)">{{ bottle.display_name }}</h1>
                    <p class="text-xs text-zinc-500 font-mono mt-0.5">{{ bottle.directory }}</p>
                    <div class="flex gap-2 mt-2">
                        <span class="text-xs px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-500">{{ bottle.scope }}</span>
                        <span class="text-xs px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-500 font-mono">{{ bottle.name }}</span>
                    </div>
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
                        :to="`/prescriptions/${rx.id}`"
                        class="flex items-center justify-between bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
                    >
                        <div class="flex items-center gap-3 min-w-0">
                            <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                                <IClipboard class="w-4 h-4 text-zinc-400" />
                            </div>
                            <div class="space-y-1 min-w-0">
                                <div class="flex items-center gap-2">
                                    <span class="text-(--text-h) text-sm font-medium">{{ rx.title }}</span>
                                    <span v-if="isOpen(rx)" class="text-xs text-green-500">● {{ $t('bottle_detail.status_open') }}</span>
                                    <span v-else-if="rx.deleted_at" class="text-xs text-red-400">● {{ $t('bottle_detail.status_deleted') }}</span>
                                    <span v-else class="text-xs text-zinc-600">● {{ $t('bottle_detail.status_closed') }}</span>
                                </div>
                                <p class="text-xs text-zinc-600">{{ new Date(rx.started_at).toLocaleString() }}</p>
                            </div>
                        </div>
                        <button
                            v-if="!rx.deleted_at"
                            class="shrink-0 text-xs text-red-400 hover:text-red-300 border border-red-900/40 px-2.5 py-1.5 rounded-lg transition-colors"
                            @click.prevent="deleteRx(rx)">
                            {{ $t('common.delete') }}
                        </button>
                    </RouterLink>
                </div>
            </div>
        </template>
    </div>
</template>
