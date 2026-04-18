<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { bottlesApi } from '@/api/bottles'
import { useActiveBottle } from '@/composables/useActiveBottle'
import type { Bottle } from '@/api/types'
import { RouterLink } from 'vue-router'

useI18n()
const { activeBottleId } = useActiveBottle()
const bottles = ref<Bottle[]>([])
const loading = ref(false)

async function load() {
    loading.value = true
    try { bottles.value = await bottlesApi.list() } finally { loading.value = false }
}

onMounted(load)
</script>

<template>
    <div class="p-6 max-w-3xl mx-auto space-y-5">
        <div class="flex items-center justify-between">
            <h1 class="text-2xl font-bold text-(--text-h)">{{ $t('nav.bottles') }}</h1>
        </div>

        <div v-if="loading" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>

        <div v-else-if="bottles.length === 0" class="text-center py-16 text-zinc-500">
            <p>{{ $t('bottles.empty') }}</p>
        </div>

        <div v-else class="space-y-2">
            <RouterLink
                v-for="b in bottles"
                :key="b.id"
                :to="`/bottles/${b.id}`"
                class="flex items-center justify-between bg-(--bg-surface) border border-(--border) rounded-lg px-4 py-3 hover:border-zinc-600 transition-colors"
            >
                <div class="min-w-0">
                    <div class="flex items-center gap-2">
                        <span class="text-(--text-h) font-medium">{{ b.display_name }}</span>
                        <span class="text-xs text-zinc-600 font-mono">{{ b.name }}</span>
                        <span class="text-xs px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-500">{{ b.scope }}</span>
                        <span v-if="b.id === activeBottleId" class="text-xs text-green-500">● {{ $t('bottles.active_badge') }}</span>
                    </div>
                    <p class="text-xs text-zinc-600 mt-0.5 truncate">{{ b.directory }}</p>
                </div>
                <div class="flex items-center gap-2 ml-4 shrink-0">
                    <button
                        v-if="b.id !== activeBottleId"
                        class="text-xs text-zinc-400 hover:text-(--text-h) transition-colors"
                        @click.prevent="activeBottleId = b.id">
                        {{ $t('bottles.activate_btn') }}
                    </button>
                    <span class="text-zinc-600">→</span>
                </div>
            </RouterLink>
        </div>
    </div>
</template>
