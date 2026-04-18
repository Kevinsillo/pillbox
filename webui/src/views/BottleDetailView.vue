<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessageBox } from 'element-plus'
import { bottlesApi } from '@/api/bottles'
import { prescriptionsApi } from '@/api/prescriptions'
import type { Bottle, Prescription } from '@/api/types'
import { RouterLink } from 'vue-router'

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
    <div class="p-6 max-w-3xl mx-auto space-y-5">
        <RouterLink to="/bottles" class="text-xs text-zinc-500 hover:text-zinc-300 transition-colors">← {{ $t('bottle_detail.back') }}</RouterLink>

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
                    <div
                        v-for="rx in prescriptions"
                        :key="rx.id"
                        class="flex items-center gap-2"
                    >
                        <RouterLink
                            :to="`/prescriptions/${rx.id}`"
                            class="flex-1 flex items-center justify-between bg-(--bg-surface) border border-(--border) rounded-lg px-4 py-3 hover:border-zinc-600 transition-colors"
                        >
                            <div>
                                <div class="flex items-center gap-2">
                                    <span class="text-(--text-h) text-sm font-medium">{{ rx.title }}</span>
                                    <span v-if="isOpen(rx)" class="text-xs text-green-500">● {{ $t('bottle_detail.status_open') }}</span>
                                    <span v-else-if="rx.deleted_at" class="text-xs text-red-500">● {{ $t('bottle_detail.status_deleted') }}</span>
                                    <span v-else class="text-xs text-zinc-600">● {{ $t('bottle_detail.status_closed') }}</span>
                                </div>
                                <p class="text-xs text-zinc-600 mt-0.5">{{ new Date(rx.started_at).toLocaleString() }}</p>
                            </div>
                            <span class="text-zinc-600">→</span>
                        </RouterLink>
                        <button
                            v-if="!rx.deleted_at"
                            class="shrink-0 text-xs text-red-400 hover:text-red-300 border border-red-900/40 px-2.5 py-1.5 rounded-lg transition-colors"
                            @click="deleteRx(rx)">
                            {{ $t('common.delete') }}
                        </button>
                    </div>
                </div>
            </div>
        </template>
    </div>
</template>
