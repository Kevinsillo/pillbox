<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessageBox } from 'element-plus'
import { capsulesApi } from '@/api/capsules'
import type { CapsuleSummary, CapsuleCompound } from '@/api/types'
import CompoundBadge from '@/components/CompoundBadge.vue'
import { RouterLink } from 'vue-router'

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
                q: searchQuery.value.trim(),
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
    <div class="p-6 max-w-3xl mx-auto space-y-5">
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

        <div v-else-if="capsules.length === 0" class="text-center py-12 text-zinc-500">
            {{ searchQuery ? $t('capsules.empty_search') : $t('capsules.empty') }}
        </div>

        <div v-else class="space-y-2">
            <div
                v-for="c in capsules"
                :key="c.id"
                class="flex items-center gap-2"
            >
                <RouterLink
                    :to="`/capsules/${c.id}`"
                    class="flex-1 flex items-center justify-between bg-(--bg-surface) border border-(--border) rounded-lg px-4 py-3 hover:border-zinc-600 transition-colors"
                >
                    <div class="flex items-center gap-3 min-w-0">
                        <CompoundBadge :compound="c.compound" />
                        <span class="text-sm text-(--text-h) truncate">{{ c.title }}</span>
                    </div>
                    <span class="text-xs text-zinc-600 shrink-0 ml-3">{{ new Date(c.updated_at).toLocaleDateString() }}</span>
                </RouterLink>
                <button
                    class="shrink-0 text-xs text-red-400 hover:text-red-300 border border-red-900/40 px-2.5 py-1.5 rounded-lg transition-colors"
                    @click="deleteCapsule(c)">
                    {{ $t('common.delete') }}
                </button>
            </div>
        </div>
    </div>
</template>
