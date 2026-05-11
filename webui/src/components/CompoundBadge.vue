<script setup lang="ts">
defineProps<{ compound: string }>()

const PALETTE = [
    'bg-(--badge-blue-bg) text-(--badge-blue-text)',
    'bg-(--badge-purple-bg) text-(--badge-purple-text)',
    'bg-(--badge-red-bg) text-(--badge-red-text)',
    'bg-(--badge-yellow-bg) text-(--badge-yellow-text)',
    'bg-(--badge-teal-bg) text-(--badge-teal-text)',
    'bg-(--badge-cyan-bg) text-(--badge-cyan-text)',
    'bg-(--badge-orange-bg) text-(--badge-orange-text)',
    'bg-(--badge-indigo-bg) text-(--badge-indigo-text)',
    'bg-(--badge-pink-bg) text-(--badge-pink-text)',
    'bg-(--badge-green-bg) text-(--badge-green-text)',
    'bg-(--badge-sky-bg) text-(--badge-sky-text)',
    'bg-(--badge-amber-bg) text-(--badge-amber-text)',
    'bg-(--badge-zinc-bg) text-(--badge-zinc-text)',
]

function compoundColor(value: string): string {
    let hash = 0
    for (let i = 0; i < value.length; i++) {
        hash = value.charCodeAt(i) + ((hash << 5) - hash)
    }
    return PALETTE[Math.abs(hash) % PALETTE.length]
}
</script>

<template>
    <span :class="['inline-block text-xs font-medium px-1.5 py-0.5 rounded', compoundColor(compound)]">
        {{ compound }}
    </span>
</template>
