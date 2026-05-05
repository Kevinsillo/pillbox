<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Pill } from '@/core/domain/types'
import { formatAuthor } from '@/core/domain/author'
import CompoundBadge from './CompoundBadge.vue'
import TruncatedTitle from './TruncatedTitle.vue'
import IFileText from '~icons/lucide/file-text'
import ITrash2 from '~icons/lucide/trash-2'
import { RouterLink } from 'vue-router'

useI18n()

defineProps<{
    pill: Pill
    bottleId: string
    editable?: boolean
}>()

defineEmits<{ delete: [] }>()
</script>

<template>
    <component
        :is="pill.prescription_id ? RouterLink : 'div'"
        :to="pill.prescription_id ? `/bottles/${bottleId}/prescriptions/${pill.prescription_id}/pills/${pill.id}` : undefined"
        class="flex items-center justify-between gap-4 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
    >
        <div class="flex items-center gap-3 min-w-0">
            <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                <IFileText class="w-4 h-4 text-zinc-400" />
            </div>
            <div class="space-y-1 min-w-0">
                <div class="flex items-center gap-2 min-w-0">
                    <CompoundBadge :compound="pill.compound" />
                    <TruncatedTitle tag="p" :title="pill.title" class="text-(--text-h) font-medium text-sm" />
                </div>
                <p class="text-xs text-zinc-600">{{ new Date(pill.updated_at).toLocaleString() }}<template v-if="formatAuthor(pill.author_name, pill.author_email)"> · {{ formatAuthor(pill.author_name, pill.author_email) }}</template></p>
            </div>
        </div>
        <div v-if="editable && pill.prescription_id" class="flex gap-2 shrink-0" @click.prevent>
            <RouterLink
                :to="`/bottles/${bottleId}/prescriptions/${pill.prescription_id}/pills/${pill.id}/edit`"
                class="text-xs text-zinc-400 hover:text-(--text-h) border border-(--border) px-2.5 py-1.5 rounded-lg transition-colors"
                @click.stop>
                {{ $t('pill_card.edit') }}
            </RouterLink>
            <button
                class="flex items-center gap-1 text-xs text-red-400 hover:text-red-300 border border-red-900/40 px-2.5 py-1.5 rounded-lg transition-colors"
                @click.stop="$emit('delete')">
                <ITrash2 class="w-3 h-3" />
                {{ $t('pill_card.delete') }}
            </button>
        </div>
    </component>
</template>
