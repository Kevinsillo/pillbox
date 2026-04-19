<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessageBox } from 'element-plus'
import { prescriptionsApi } from '@/core/infrastructure/repositories/PrescriptionsRepository'
import { pillsApi } from '@/core/infrastructure/repositories/PillsRepository'
import type { Prescription, Pill } from '@/core/domain/types'
import PillCard from '@/components/PillCard.vue'
import IArrowLeft from '~icons/lucide/arrow-left'

const { t } = useI18n()
const props = defineProps<{ id: string }>()
const router = useRouter()

const rx = ref<Prescription | null>(null)
const pills = ref<Pill[]>([])
const loading = ref(false)

async function load() {
    loading.value = true
    try {
        const [r, p] = await Promise.all([
            prescriptionsApi.get(props.id),
            prescriptionsApi.pills(props.id),
        ])
        rx.value = r
        pills.value = p.sort((a, b) => b.created_at.localeCompare(a.created_at))
    } finally {
        loading.value = false
    }
}

onMounted(load)

const isOpen = computed(() => rx.value?.ended_at === null && rx.value?.deleted_at === null)

async function deletePill(pill: Pill) {
    try {
        await ElMessageBox.confirm(
            t('confirm.delete_pill_msg', { title: pill.title }),
            t('confirm.delete_pill_title'),
            {
                confirmButtonText: t('common.delete'),
                cancelButtonText: t('common.cancel'),
                type: 'warning',
            }
        )
        await pillsApi.delete(pill.id)
        pills.value = pills.value.filter(p => p.id !== pill.id)
    } catch { /* cancelled */ }
}

async function closeRx() {
    try {
        await ElMessageBox.confirm(
            t('confirm.close_prescription_msg'),
            t('confirm.close_prescription_title'),
            {
                confirmButtonText: t('prescription_detail.close_btn'),
                cancelButtonText: t('common.cancel'),
                type: 'warning',
            }
        )
        const updated = await prescriptionsApi.close(props.id)
        rx.value = updated
    } catch { /* cancelled */ }
}

async function deleteRx() {
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
        await prescriptionsApi.delete(props.id)
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
                <div>
                    <div class="flex items-center gap-2 mb-1">
                        <span v-if="isOpen" class="text-xs text-green-500">● {{ $t('prescription_detail.status_open') }}</span>
                        <span v-else class="text-xs text-zinc-600">● {{ $t('prescription_detail.status_closed') }}</span>
                    </div>
                    <h1 class="text-xl font-bold text-(--text-h)">{{ rx.title }}</h1>
                    <p class="text-xs text-zinc-500 mt-0.5">
                        {{ $t('prescription_detail.started_at') }} {{ new Date(rx.started_at).toLocaleString() }}
                        <span v-if="rx.ended_at"> · {{ $t('prescription_detail.closed_at') }} {{ new Date(rx.ended_at).toLocaleString() }}</span>
                    </p>
                </div>
                <div class="flex gap-2">
                    <button v-if="isOpen"
                            class="text-sm text-zinc-400 hover:text-(--text-h) border border-(--border) px-3 py-2 rounded-lg transition-colors"
                            @click="closeRx">
                        {{ $t('prescription_detail.close_btn') }}
                    </button>
                    <button class="text-sm text-red-400 hover:text-red-300 border border-red-900/40 px-3 py-2 rounded-lg transition-colors"
                            @click="deleteRx">
                        {{ $t('common.delete') }}
                    </button>
                </div>
            </div>

            <!-- Pills -->
            <div>
                <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">
                    {{ $t('prescription_detail.pills_heading') }} ({{ pills.length }})
                </h2>
                <div v-if="pills.length === 0" class="text-zinc-500 text-sm">{{ $t('prescription_detail.empty') }}</div>
                <div v-else class="space-y-3">
                    <PillCard
                        v-for="pill in pills"
                        :key="pill.id"
                        :pill="pill"
                        :editable="isOpen"
                        @delete="deletePill(pill)"
                    />
                </div>
            </div>
        </template>
    </div>
</template>
