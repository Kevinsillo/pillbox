<script setup lang="ts">
import type { PillCompound, CapsuleCompound } from '@/core/domain/types'

defineProps<{ compound: PillCompound | CapsuleCompound }>()

const colors: Record<string, string> = {
    decision:             'bg-(--badge-blue-bg) text-(--badge-blue-text)',
    architecture:         'bg-(--badge-purple-bg) text-(--badge-purple-text)',
    bugfix:               'bg-(--badge-red-bg) text-(--badge-red-text)',
    pattern:              'bg-(--badge-yellow-bg) text-(--badge-yellow-text)',
    discovery:            'bg-(--badge-teal-bg) text-(--badge-teal-text)',
    learning:             'bg-(--badge-cyan-bg) text-(--badge-cyan-text)',
    feedback:             'bg-(--badge-orange-bg) text-(--badge-orange-text)',
    prescription_summary: 'bg-(--badge-zinc-bg) text-(--badge-zinc-text)',
    manual:               'bg-(--badge-zinc-bg) text-(--badge-zinc-alt-text)',
    convention:           'bg-(--badge-indigo-bg) text-(--badge-indigo-text)',
    workflow:             'bg-(--badge-pink-bg) text-(--badge-pink-text)',
    environment:          'bg-(--badge-green-bg) text-(--badge-green-text)',
    context:              'bg-(--badge-sky-bg) text-(--badge-sky-text)',
    goal:                 'bg-(--badge-amber-bg) text-(--badge-amber-text)',
}
</script>

<template>
    <span :class="['inline-block text-xs font-medium px-1.5 py-0.5 rounded', colors[compound] ?? 'bg-(--badge-zinc-bg) text-(--badge-zinc-alt-text)']">
        {{ compound }}
    </span>
</template>
