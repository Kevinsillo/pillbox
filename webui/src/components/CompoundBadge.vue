<script setup lang="ts">
import type { PillCompound, CapsuleCompound } from '@/core/domain/types'

defineProps<{ compound: PillCompound | CapsuleCompound }>()

const colors: Record<string, string> = {
    decision:             'bg-blue-900/50 text-blue-300',
    architecture:         'bg-purple-900/50 text-purple-300',
    bugfix:               'bg-red-900/50 text-red-300',
    pattern:              'bg-yellow-900/50 text-yellow-300',
    discovery:            'bg-teal-900/50 text-teal-300',
    learning:             'bg-cyan-900/50 text-cyan-300',
    feedback:             'bg-orange-900/50 text-orange-300',
    prescription_summary: 'bg-zinc-700/50 text-zinc-300',
    manual:               'bg-zinc-700/50 text-zinc-400',
    convention:           'bg-indigo-900/50 text-indigo-300',
    workflow:             'bg-pink-900/50 text-pink-300',
    environment:          'bg-green-900/50 text-green-300',
    context:              'bg-sky-900/50 text-sky-300',
    goal:                 'bg-amber-900/50 text-amber-300',
}
</script>

<template>
    <span :class="['inline-block text-xs font-medium px-1.5 py-0.5 rounded', colors[compound] ?? 'bg-zinc-700 text-zinc-400']">
        {{ compound }}
    </span>
</template>
