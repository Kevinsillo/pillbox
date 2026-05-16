<script setup lang="ts">
defineProps<{ title: string }>()
defineEmits<{ close: [] }>()
</script>

<template>
    <Teleport to="body">
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
             @click.self="$emit('close')">
            <div class="bg-(--bg-surface) border border-(--border) rounded-xl w-full max-w-lg shadow-2xl">
                <div class="flex items-center justify-between px-5 py-4 border-b border-(--border)">
                    <h2 class="text-(--text-h) font-semibold">{{ title }}</h2>
                    <button class="text-(--text) hover:text-(--text-h)"
                            @click="$emit('close')">✕</button>
                </div>
                <div class="p-5">
                    <slot />
                </div>
            </div>
        </div>
    </Teleport>
</template>
