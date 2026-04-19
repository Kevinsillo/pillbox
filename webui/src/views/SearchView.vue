<script setup lang="ts">
import CompoundBadge from "@/components/CompoundBadge.vue"
import { useActiveBottle } from "@/composables/useActiveBottle"
import type { CapsuleSearchResult, PillSearchResult } from "@/core/domain/types"
import { capsulesApi } from "@/core/infrastructure/repositories/CapsulesRepository"
import { pillsApi } from "@/core/infrastructure/repositories/PillsRepository"
import { ref } from "vue"
import { useI18n } from "vue-i18n"
import { RouterLink } from "vue-router"
import IFileText from "~icons/lucide/file-text"
import IPill from "~icons/lucide/pill"

useI18n()

const { activeBottleId } = useActiveBottle()

const query = ref("")
const pillResults = ref<PillSearchResult[]>([])
const capsuleResults = ref<CapsuleSearchResult[]>([])
const loading = ref(false)
const searched = ref(false)

let debounce: ReturnType<typeof setTimeout>

async function search() {
    const q = query.value.trim()
    if (!q) {
        pillResults.value = []
        capsuleResults.value = []
        searched.value = false
        return
    }

    loading.value = true
    searched.value = true
    try {
        const [pills, capsules] = await Promise.all([
            pillsApi.search({
                query: q,
                bottle_id: activeBottleId.value ?? undefined,
                limit: 20,
            }),
            capsulesApi.search({ query: q, limit: 20 }),
        ])
        pillResults.value = pills
        capsuleResults.value = capsules
    } finally {
        loading.value = false
    }
}

function onInput() {
    clearTimeout(debounce)
    debounce = setTimeout(search, 150)
}
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <h1 class="text-2xl font-bold text-(--text-h)">{{ $t("search.heading") }}</h1>

        <input
            v-model="query"
            :placeholder="$t('search.placeholder')"
            autofocus
            class="w-full bg-(--bg-surface) border border-(--border) rounded-lg px-4 py-3 text-(--text-h) focus:outline-none focus:border-zinc-500 text-sm"
            @input="onInput"
        />

        <div v-if="loading" class="text-center py-12 text-zinc-500">{{ $t("search.searching") }}…</div>

        <template v-else-if="searched">
            <!-- Capsules -->
            <div v-if="capsuleResults.length > 0">
                <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">
                    {{ $t("search.capsules_heading") }} ({{ capsuleResults.length }})
                </h2>
                <div class="space-y-2">
                    <RouterLink
                        v-for="c in capsuleResults"
                        :key="c.id"
                        :to="`/capsules/${c.id}`"
                        class="flex items-start gap-3 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
                    >
                        <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                            <IPill class="w-4 h-4 text-zinc-400" />
                        </div>
                        <div class="min-w-0">
                            <div class="flex items-center gap-2 mb-1">
                                <CompoundBadge :compound="c.compound" />
                                <span class="text-sm text-(--text-h) font-medium truncate">{{ c.title }}</span>
                            </div>
                            <p class="text-xs text-zinc-500 line-clamp-2" v-html="c.snippet" />
                            <p class="text-xs text-zinc-600 mt-1">{{ new Date(c.updated_at).toLocaleDateString() }}</p>
                        </div>
                    </RouterLink>
                </div>
            </div>

            <!-- Pills -->
            <div v-if="pillResults.length > 0">
                <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">
                    {{ $t("search.pills_heading") }} ({{ pillResults.length }})
                </h2>
                <div class="space-y-2">
                    <RouterLink
                        v-for="p in pillResults"
                        :key="p.id"
                        :to="`/pills/${p.id}`"
                        class="flex items-start gap-3 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
                    >
                        <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                            <IFileText class="w-4 h-4 text-zinc-400" />
                        </div>
                        <div class="min-w-0">
                            <div class="flex items-center gap-2 mb-1">
                                <CompoundBadge :compound="p.compound" />
                                <span class="text-sm text-(--text-h) font-medium truncate">{{ p.title }}</span>
                            </div>
                            <p class="text-xs text-zinc-500 line-clamp-2" v-html="p.snippet" />
                            <p class="text-xs text-zinc-600 mt-1">{{ new Date(p.updated_at).toLocaleDateString() }}</p>
                        </div>
                    </RouterLink>
                </div>
            </div>

            <div v-if="pillResults.length === 0 && capsuleResults.length === 0" class="text-center py-12 text-zinc-500">
                {{ $t("search.no_results", { query }) }}
            </div>
        </template>

        <div v-else class="text-center py-12 text-zinc-600 text-sm">
            {{ $t("search.hint") }}
        </div>
    </div>
</template>
