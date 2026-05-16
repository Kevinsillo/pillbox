<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { Prescription } from '@/core/domain/types'
import { formatAuthor } from '@/core/domain/author'
import { shortId } from '@/core/utils/id'
import PrescriptionStatusBadge from './PrescriptionStatusBadge.vue'
import TruncatedTitle from './TruncatedTitle.vue'
import IClipboard from '~icons/lucide/clipboard'

useI18n()

const props = defineProps<{
    prescription: Prescription
    bottleId: string
    archived?: boolean
}>()

const isOpen = computed(() =>
    props.prescription.ended_at === null && props.prescription.deleted_at === null
)
</script>

<template>
    <div :class="['relative', archived && 'opacity-50']">
        <RouterLink
            :to="`/bottles/${shortId(bottleId)}/prescriptions/${shortId(prescription.id)}`"
            class="flex items-center gap-3 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600"
        >
            <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                <IClipboard class="w-4 h-4 text-zinc-400" />
            </div>
            <div class="space-y-1 min-w-0">
                <div class="flex items-center gap-2 min-w-0">
                    <PrescriptionStatusBadge :open="isOpen" />
                    <TruncatedTitle :title="prescription.title" class="text-(--text-h) text-sm font-medium" />
                </div>
                <p class="text-xs text-zinc-600">
                    {{ new Date(prescription.started_at).toLocaleString() }}<template v-if="formatAuthor(prescription.author_name, prescription.author_email)"> · {{ formatAuthor(prescription.author_name, prescription.author_email) }}</template>
                </p>
            </div>
        </RouterLink>
        <span
            v-if="archived"
            class="absolute bottom-2 right-2 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-(--badge-zinc-bg) text-(--badge-zinc-text) pointer-events-none"
        >{{ $t('prescription_detail.archived_badge') }}</span>
    </div>
</template>
