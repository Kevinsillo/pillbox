<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessageBox } from 'element-plus'
import { prescriptionsApi } from '@/api/prescriptions'
import { pillsApi } from '@/api/pills'
import type { Prescription, Pill, PillCompound } from '@/api/types'
import PillCard from '@/components/PillCard.vue'
import AppModal from '@/components/AppModal.vue'

const { t } = useI18n()
const props = defineProps<{ id: string }>()
const router = useRouter()

const rx = ref<Prescription | null>(null)
const pills = ref<Pill[]>([])
const loading = ref(false)

const showPillModal = ref(false)
const editingPill = ref<Pill | null>(null)
const pillForm = ref({ title: '', content: '', compound: 'manual' as PillCompound })
const saving = ref(false)
const formError = ref<string | null>(null)

const PILL_COMPOUNDS: PillCompound[] = [
    'decision', 'architecture', 'bugfix', 'pattern', 'discovery',
    'learning', 'feedback', 'prescription_summary', 'manual',
]

async function load() {
    loading.value = true
    try {
        const [r, p] = await Promise.all([
            prescriptionsApi.get(props.id),
            prescriptionsApi.pills(props.id),
        ])
        rx.value = r
        pills.value = p
    } finally {
        loading.value = false }
}

onMounted(load)

const isOpen = computed(() => rx.value?.ended_at === null && rx.value?.deleted_at === null)

function openEditPill(pill: Pill) {
    editingPill.value = pill
    pillForm.value = { title: pill.title, content: pill.content, compound: pill.compound }
    showPillModal.value = true
}

async function savePill() {
    if (!editingPill.value) return
    saving.value = true
    formError.value = null
    try {
        const updated = await pillsApi.update(editingPill.value.id, pillForm.value)
        const idx = pills.value.findIndex(p => p.id === editingPill.value!.id)
        if (idx !== -1) pills.value[idx] = updated
        showPillModal.value = false
    } catch (e: unknown) {
        formError.value = e instanceof Error ? e.message : 'Error'
    } finally {
        saving.value = false
    }
}

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
    <div class="p-6 max-w-3xl mx-auto space-y-5">
        <button class="text-xs text-zinc-500 hover:text-zinc-300 transition-colors" @click="router.back()">← {{ $t('common.back') }}</button>

        <div v-if="loading" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>

        <template v-else-if="rx">
            <!-- Header -->
            <div class="flex items-start justify-between gap-4">
                <div>
                    <div class="flex items-center gap-2 mb-1">
                        <span v-if="isOpen" class="text-xs text-green-500">● {{ $t('prescription_detail.status_open') }}</span>
                        <span v-else class="text-xs text-zinc-500">● {{ $t('prescription_detail.status_closed') }}</span>
                    </div>
                    <h1 class="text-xl font-bold text-(--text-h)">{{ rx.title }}</h1>
                    <p class="text-xs text-zinc-500 mt-0.5">
                        {{ $t('prescription_detail.started_at') }} {{ new Date(rx.started_at).toLocaleString() }}
                        <span v-if="rx.ended_at"> · {{ $t('prescription_detail.closed_at') }} {{ new Date(rx.ended_at).toLocaleString() }}</span>
                    </p>
                </div>
                <div class="flex gap-2 shrink-0">
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
                        @edit="openEditPill(pill)"
                        @delete="deletePill(pill)"
                    />
                </div>
            </div>
        </template>

        <!-- Modal editar pill -->
        <AppModal
            v-if="showPillModal"
            :title="$t('prescription_detail.modal_edit_title')"
            @close="showPillModal = false"
        >
            <form class="space-y-3" @submit.prevent="savePill">
                <div>
                    <label class="block text-xs text-zinc-400 mb-1">{{ $t('common.compound') }}</label>
                    <select v-model="pillForm.compound"
                            class="w-full bg-(--bg) border border-(--border) rounded-md px-3 py-2 text-sm text-(--text-h) focus:outline-none focus:border-zinc-500">
                        <option v-for="c in PILL_COMPOUNDS" :key="c" :value="c">{{ c }}</option>
                    </select>
                </div>
                <div>
                    <label class="block text-xs text-zinc-400 mb-1">{{ $t('common.title') }}</label>
                    <input v-model="pillForm.title" required maxlength="255"
                           class="w-full bg-(--bg) border border-(--border) rounded-md px-3 py-2 text-sm text-(--text-h) focus:outline-none focus:border-zinc-500" />
                </div>
                <div>
                    <label class="block text-xs text-zinc-400 mb-1">{{ $t('common.content_md') }}</label>
                    <textarea v-model="pillForm.content" required rows="6" maxlength="5000"
                              class="w-full bg-(--bg) border border-(--border) rounded-md px-3 py-2 text-sm text-(--text-h) focus:outline-none focus:border-zinc-500 resize-y font-mono" />
                </div>
                <p v-if="formError" class="text-red-400 text-xs">{{ formError }}</p>
                <div class="flex justify-end gap-2 pt-1">
                    <button type="button" @click="showPillModal = false"
                            class="text-sm text-zinc-400 hover:text-(--text-h) px-3 py-2 transition-colors">
                        {{ $t('common.cancel') }}
                    </button>
                    <button type="submit" :disabled="saving"
                            class="bg-(--accent-bg) hover:bg-zinc-600 disabled:opacity-50 text-(--text-h) text-sm px-4 py-2 rounded-lg transition-colors">
                        {{ saving ? ($t('common.saving') + '…') : $t('common.save') }}
                    </button>
                </div>
            </form>
        </AppModal>
    </div>
</template>
