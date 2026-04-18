<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Pill } from '@/api/types'
import CompoundBadge from './CompoundBadge.vue'
import { marked } from 'marked'
import { computed } from 'vue'

useI18n()

const props = defineProps<{
    pill: Pill
    editable?: boolean
}>()

defineEmits<{ edit: []; delete: [] }>()

const renderedContent = computed(() => marked.parse(props.pill.content) as string)
</script>

<template>
    <div class="bg-(--bg-surface) border border-(--border) rounded-lg p-4 space-y-2">
        <div class="flex items-start justify-between gap-2">
            <div class="flex items-center gap-2 flex-wrap">
                <CompoundBadge :compound="pill.compound" />
                <span class="text-(--text-h) font-medium text-sm">{{ pill.title }}</span>
            </div>
            <div v-if="editable" class="flex gap-2 shrink-0">
                <button class="text-xs text-(--text) hover:text-(--text-h) transition-colors"
                        @click="$emit('edit')">{{ $t('pill_card.edit') }}</button>
                <button class="text-xs text-red-400 hover:text-red-300 transition-colors"
                        @click="$emit('delete')">{{ $t('pill_card.delete') }}</button>
            </div>
        </div>
        <div class="markdown text-sm" v-html="renderedContent" />
        <p class="text-xs text-zinc-600">{{ new Date(pill.updated_at).toLocaleString() }}</p>
    </div>
</template>
