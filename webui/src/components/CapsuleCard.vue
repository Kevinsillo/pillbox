<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'
import type { CapsuleSummary } from '@/core/domain/types'
import { shortId } from '@/core/utils/id'
import { formatDateTime } from '@/core/utils/date'
import CompoundBadge from './CompoundBadge.vue'
import TruncatedTitle from './TruncatedTitle.vue'
import ViewsBadge from './ViewsBadge.vue'
import IPill from '~icons/lucide/pill'

useI18n()

defineProps<{
    capsule: CapsuleSummary
    archived?: boolean
}>()
</script>

<template>
    <div class="relative" :class="{ 'opacity-50': archived }">
        <RouterLink
            :to="`/capsules/${shortId(capsule.id)}`"
            class="flex items-center gap-3 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
        >
            <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                <IPill class="w-4 h-4 text-zinc-400" />
            </div>
            <div class="space-y-1 min-w-0">
                <div class="flex items-center gap-2 min-w-0">
                    <CompoundBadge :compound="capsule.compound" />
                    <TruncatedTitle :title="capsule.title" class="text-sm text-(--text-h) font-medium" />
                </div>
                <p class="text-xs text-zinc-600 flex items-center gap-2 flex-wrap">
                    <ViewsBadge :views="capsule.views" />
                    <span>{{ formatDateTime(capsule.updated_at) }}</span>
                </p>
            </div>
        </RouterLink>
        <span
            v-if="archived"
            class="absolute bottom-2 right-2 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-(--badge-zinc-bg) text-(--badge-zinc-text) pointer-events-none"
        >{{ $t('capsules.archived_badge') }}</span>
    </div>
</template>
