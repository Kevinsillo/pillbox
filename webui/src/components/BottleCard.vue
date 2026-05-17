<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'
import type { Bottle } from '@/core/domain/types'
import { shortId } from '@/core/utils/id'
import ViewsBadge from './ViewsBadge.vue'
import IBox from '~icons/lucide/box'

useI18n()

defineProps<{
    bottle: Bottle
    active?: boolean
}>()

defineEmits<{ activate: [] }>()
</script>

<template>
    <RouterLink
        :to="`/bottles/${shortId(bottle.id)}`"
        class="flex items-center justify-between bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
    >
        <div class="flex items-center gap-3 min-w-0">
            <div class="relative shrink-0">
                <div
                    :class="active ? 'bg-green-500/5 border border-green-500/50' : 'bg-(--accent-bg)'"
                    class="w-9 h-9 rounded-lg flex items-center justify-center"
                >
                    <IBox :class="active ? 'text-green-500' : 'text-zinc-400'" class="w-4 h-4" />
                </div>
                <span
                    v-if="active"
                    class="absolute -top-1 -right-1 w-2.5 h-2.5 rounded-full bg-green-500 border-2 border-(--bg-surface)"
                />
            </div>
            <div class="min-w-0">
                <div class="flex items-baseline gap-2">
                    <span class="text-(--text-h) font-medium">{{ bottle.display_name }}</span>
                    <span class="text-sm text-zinc-600 font-mono">{{ bottle.name }}</span>
                </div>
                <div class="flex items-center gap-2 mt-0.5 min-w-0 text-xs text-zinc-600">
                    <ViewsBadge :views="bottle.views" />
                    <span class="text-[11px] border border-(--border) px-1.5 py-0 rounded shrink-0">{{ bottle.scope }}</span>
                    <span class="truncate">{{ bottle.directory }}</span>
                </div>
            </div>
        </div>
        <button
            v-if="!active"
            class="text-xs text-zinc-400 hover:text-(--text-h) ml-4 shrink-0 transition-colors"
            @click.prevent="$emit('activate')"
        >
            {{ $t('bottles.activate_btn') }}
        </button>
    </RouterLink>
</template>
