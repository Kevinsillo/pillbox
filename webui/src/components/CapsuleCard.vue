<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'
import type { CapsuleSummary } from '@/core/domain/types'
import { shortId } from '@/core/utils/id'
import CompoundBadge from './CompoundBadge.vue'
import TruncatedTitle from './TruncatedTitle.vue'
import ViewsBadge from './ViewsBadge.vue'
import IPill from '~icons/lucide/pill'
import ITrash2 from '~icons/lucide/trash-2'

useI18n()

defineProps<{
    capsule: CapsuleSummary
    archived?: boolean
}>()

defineEmits<{
    archive: []
    purge: []
}>()
</script>

<template>
    <RouterLink
        :to="`/capsules/${shortId(capsule.id)}`"
        class="flex items-center justify-between gap-4 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
    >
        <div class="flex items-center gap-3 min-w-0">
            <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                <IPill class="w-4 h-4 text-zinc-400" />
            </div>
            <div class="space-y-1 min-w-0">
                <div class="flex items-center gap-2 min-w-0">
                    <span
                        v-if="archived"
                        class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-zinc-700/50 text-zinc-400"
                    >{{ $t('capsules.archived_badge') }}</span>
                    <CompoundBadge :compound="capsule.compound" />
                    <TruncatedTitle :title="capsule.title" class="text-sm text-(--text-h) font-medium" />
                </div>
                <p class="text-xs text-zinc-600 flex items-center gap-2 flex-wrap">
                    <ViewsBadge :views="capsule.views" />
                    <span>{{ new Date(capsule.updated_at).toLocaleString() }}</span>
                </p>
            </div>
        </div>
        <div class="flex gap-2 shrink-0" @click.prevent>
            <RouterLink
                v-if="!archived"
                :to="`/capsules/${shortId(capsule.id)}`"
                class="text-xs text-zinc-400 hover:text-(--text-h) border border-(--border) px-2.5 py-1.5 rounded-lg transition-colors"
                @click.stop
            >
                {{ $t('common.edit') }}
            </RouterLink>
            <button
                v-if="!archived"
                class="flex items-center gap-1 text-xs text-red-400 hover:text-red-300 border border-red-900/40 px-2.5 py-1.5 rounded-lg transition-colors"
                @click.stop="$emit('archive')"
            >
                <ITrash2 class="w-3 h-3" />
                {{ $t('common.delete') }}
            </button>
            <button
                v-if="archived"
                class="flex items-center gap-1 text-xs text-red-400 hover:text-red-300 border border-red-900/40 px-2.5 py-1.5 rounded-lg transition-colors"
                @click.stop="$emit('purge')"
            >
                <ITrash2 class="w-3 h-3" />
                {{ $t('common.delete_permanent') }}
            </button>
        </div>
    </RouterLink>
</template>
