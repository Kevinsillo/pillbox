<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessageBox } from 'element-plus'
import { capsulesApi } from '@/core/infrastructure/repositories/CapsulesRepository'
import type { CapsuleSummary, CapsuleCompound } from '@/core/domain/types'
import CompoundBadge from '@/components/CompoundBadge.vue'
import { RouterLink } from 'vue-router'
import IPill from '~icons/lucide/pill'

const { t } = useI18n()

const CAPSULE_COMPOUNDS: CapsuleCompound[] = ['convention', 'workflow', 'environment', 'context', 'goal', 'feedback', 'manual']

const capsules = ref<CapsuleSummary[]>([])
const loading = ref(false)
const filterCompound = ref<CapsuleCompound | ''>('')
const searchQuery = ref('')

async function load() {
    loading.value = true
    try {
        if (searchQuery.value.trim()) {
            capsules.value = await capsulesApi.search({
                query: searchQuery.value.trim(),
                compound: filterCompound.value || undefined,
            })
        } else {
            capsules.value = await capsulesApi.list({
                compound: filterCompound.value || undefined,
            })
        }
    } finally {
        loading.value = false }
}

onMounted(load)

let debounce: ReturnType<typeof setTimeout>
function onSearch() {
    clearTimeout(debounce)
    debounce = setTimeout(load, 300)
}

async function deleteCapsule(c: CapsuleSummary) {
    try {
        await ElMessageBox.confirm(
            t('confirm.delete_capsule_msg', { title: c.title }),
            t('confirm.delete_capsule_title'),
            {
                confirmButtonText: t('common.delete'),
                cancelButtonText: t('common.cancel'),
                type: 'warning',
            }
        )
        await capsulesApi.delete(c.id)
        capsules.value = capsules.value.filter(x => x.id !== c.id)
    } catch { /* cancelled */ }
}
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <div class="flex items-center justify-between">
            <h1 class="text-2xl font-bold text-(--text-h)">{{ $t('nav.capsules') }}</h1>
        </div>

        <!-- Filtros -->
        <div class="flex gap-3">
            <input
                v-model="searchQuery"
                :placeholder="$t('capsules.search_placeholder')"
                class="flex-1 bg-(--bg-surface) border border-(--border) rounded-lg px-3 py-2 text-sm text-(--text-h) focus:outline-none focus:border-zinc-500"
                @input="onSearch"
            />
            <select
                v-model="filterCompound"
                class="bg-(--bg-surface) border border-(--border) rounded-lg px-3 py-2 text-sm text-(--text-h) focus:outline-none focus:border-zinc-500"
                @change="load"
            >
                <option value="">{{ $t('capsules.filter_all') }}</option>
                <option v-for="c in CAPSULE_COMPOUNDS" :key="c" :value="c">{{ c }}</option>
            </select>
        </div>

        <div v-if="loading" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>

        <div v-else-if="capsules.length === 0" class="text-center py-16 text-zinc-500">
            {{ searchQuery ? $t('capsules.empty_search') : $t('capsules.empty') }}
        </div>

        <div v-else class="space-y-2">
            <RouterLink
                v-for="c in capsules"
                :key="c.id"
                :to="`/capsules/${c.id}`"
                class="flex items-center justify-between gap-4 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
            >
                <div class="flex items-center gap-3 min-w-0">
                    <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                        <IPill class="w-4 h-4 text-zinc-400" />
                    </div>
                    <div class="space-y-1 min-w-0">
                        <div class="flex items-center gap-2">
                            <CompoundBadge :compound="c.compound" />
                            <span class="text-sm text-(--text-h) font-medium truncate">{{ c.title }}</span>
                        </div>
                        <p class="text-xs text-zinc-600">{{ new Date(c.updated_at).toLocaleString() }}</p>
                    </div>
                </div>
                <div class="flex gap-2 shrink-0" @click.prevent>
                    <RouterLink
                        :to="`/capsules/${c.id}`"
                        class="text-xs text-zinc-400 hover:text-(--text-h) border border-(--border) px-2.5 py-1.5 rounded-lg transition-colors"
                        @click.stop>
                        {{ $t('common.edit') }}
                    </RouterLink>
                    <button
                        class="text-xs text-red-400 hover:text-red-300 border border-red-900/40 px-2.5 py-1.5 rounded-lg transition-colors"
                        @click.stop="deleteCapsule(c)">
                        {{ $t('common.delete') }}
                    </button>
                </div>
            </RouterLink>
        </div>
    </div>
</template>
