<script setup lang="ts">
import CompoundBadge from "@/components/CompoundBadge.vue"
import TruncatedTitle from "@/components/TruncatedTitle.vue"
import Paginator from "@/components/Paginator.vue"
import LoadingState from "@/components/LoadingState.vue"
import EmptyState from "@/components/EmptyState.vue"
import { useActiveBottle } from "@/composables/useActiveBottle"
import type { CapsuleSearchResult, Compound, PillSearchResult } from "@/core/domain/types"
import { capsulesApi } from "@/core/infrastructure/repositories/CapsulesRepository"
import { pillsApi } from "@/core/infrastructure/repositories/PillsRepository"
import { shortId } from "@/core/utils/id"
import { formatDate } from "@/core/utils/date"
import { ElAlert, ElInput, ElOption, ElRadioButton, ElRadioGroup, ElSelect } from "element-plus"
import { onMounted, ref, watch } from "vue"
import { useI18n } from "vue-i18n"
import { RouterLink } from "vue-router"
import IFileText from "~icons/lucide/file-text"
import IPill from "~icons/lucide/pill"

const { t } = useI18n()

const { activeBottleId } = useActiveBottle()

type Scope = "all" | "pills" | "capsules"

const query = ref("")
const scope = ref<Scope>("all")
const compound = ref<string>("")
const compounds = ref<Compound[]>([])
const pillItems = ref<PillSearchResult[]>([])
const pillsTotal = ref(0)
const capItems = ref<CapsuleSearchResult[]>([])
const capsTotal = ref(0)
const loading = ref(false)
const searched = ref(false)
const isFuzzy = ref(false)

const pillsPage = ref(1)
const pillsPageSize = ref(20)
const capsPage = ref(1)
const capsPageSize = ref(20)

let debounce: ReturnType<typeof setTimeout>
let currentController: AbortController | null = null

async function loadCompounds() {
    try {
        if (scope.value === "all") {
            const [pillsC, capsC] = await Promise.all([
                pillsApi.getCompounds(activeBottleId.value ?? undefined),
                capsulesApi.getCompounds(),
            ])
            const map = new Map<string, number>()
            for (const c of pillsC) map.set(c.compound, (map.get(c.compound) ?? 0) + c.count)
            for (const c of capsC) map.set(c.compound, (map.get(c.compound) ?? 0) + c.count)
            compounds.value = Array.from(map.entries())
                .map(([cmp, count]) => ({ compound: cmp, count }))
                .sort((a, b) => b.count - a.count || a.compound.localeCompare(b.compound))
        } else if (scope.value === "pills") {
            compounds.value = await pillsApi.getCompounds(activeBottleId.value ?? undefined)
        } else {
            compounds.value = await capsulesApi.getCompounds()
        }
    } catch {
        compounds.value = []
    }
}

async function runSearch() {
    const q = query.value.trim()
    isFuzzy.value = false

    if (!q && !compound.value) {
        pillItems.value = []
        pillsTotal.value = 0
        capItems.value = []
        capsTotal.value = 0
        searched.value = false
        return
    }

    if (currentController) currentController.abort()
    currentController = new AbortController()

    loading.value = true
    searched.value = true

    const effectiveQuery = q
    const effectiveCompound = compound.value || undefined

    const runPills = () =>
        pillsApi.search({
            query: effectiveQuery,
            bottle_id: activeBottleId.value ?? undefined,
            compound: effectiveCompound,
            page: pillsPage.value,
            page_size: pillsPageSize.value,
        })
    const runCaps = () =>
        capsulesApi.search({
            query: effectiveQuery,
            compound: effectiveCompound,
            page: capsPage.value,
            page_size: capsPageSize.value,
        })

    try {
        // El backend decide internamente si reintenta con fuzzy y nos
        // devuelve `used_fuzzy` en la respuesta paginada.
        let pillsRes: { items: PillSearchResult[]; total: number; used_fuzzy?: boolean } = {
            items: [],
            total: 0,
        }
        let capsRes: { items: CapsuleSearchResult[]; total: number; used_fuzzy?: boolean } = {
            items: [],
            total: 0,
        }
        if (scope.value === "all") {
            ;[pillsRes, capsRes] = await Promise.all([runPills(), runCaps()])
        } else if (scope.value === "pills") {
            pillsRes = await runPills()
        } else {
            capsRes = await runCaps()
        }

        isFuzzy.value = Boolean(pillsRes.used_fuzzy || capsRes.used_fuzzy)
        pillItems.value = pillsRes.items
        pillsTotal.value = pillsRes.total
        capItems.value = capsRes.items
        capsTotal.value = capsRes.total
    } finally {
        loading.value = false
    }
}

function triggerSearch() {
    clearTimeout(debounce)
    debounce = setTimeout(runSearch, 150)
}

function resetPages() {
    pillsPage.value = 1
    capsPage.value = 1
}

function onInput() {
    isFuzzy.value = false
    resetPages()
    triggerSearch()
}

function onScopeChange() {
    isFuzzy.value = false
    compound.value = ""
    resetPages()
    loadCompounds()
    triggerSearch()
}

function onCompoundChange() {
    isFuzzy.value = false
    resetPages()
    triggerSearch()
}

watch(activeBottleId, () => {
    resetPages()
    loadCompounds()
    triggerSearch()
})

watch(pillsPage, () => {
    runSearch()
})

watch(capsPage, () => {
    runSearch()
})

onMounted(() => {
    loadCompounds()
})
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <h1 class="text-2xl font-bold text-(--text-h)">{{ $t("search.heading") }}</h1>

        <div class="flex flex-wrap gap-3 items-center">
            <el-radio-group v-model="scope" @change="onScopeChange">
                <el-radio-button value="all">{{ t("search.scope.all") }}</el-radio-button>
                <el-radio-button value="pills">{{ t("search.scope.pills") }}</el-radio-button>
                <el-radio-button value="capsules">{{ t("search.scope.capsules") }}</el-radio-button>
            </el-radio-group>

            <el-select
                v-model="compound"
                filterable
                clearable
                :placeholder="t('search.compound_placeholder')"
                class="min-w-55"
                @change="onCompoundChange"
                @clear="onCompoundChange"
            >
                <el-option
                    v-for="c in compounds"
                    :key="c.compound"
                    :label="`${c.compound} (${c.count})`"
                    :value="c.compound"
                />
            </el-select>

            <el-input
                v-model="query"
                :placeholder="$t('search.placeholder')"
                autofocus
                class="w-full"
                @input="onInput"
            />
        </div>

        <el-alert
            v-if="isFuzzy"
            :title="t('search.fuzzy_banner')"
            type="info"
            show-icon
            :closable="false"
        />

        <LoadingState v-if="loading" :text="$t('search.searching')" padding="py-12" />

        <template v-else-if="searched">
            <!-- Capsules -->
            <div v-if="(scope === 'all' || scope === 'capsules') && capItems.length > 0">
                <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">
                    {{ $t("search.capsules_heading") }} ({{ capsTotal }})
                </h2>
                <div class="space-y-2">
                    <RouterLink
                        v-for="c in capItems"
                        :key="c.id"
                        :to="`/capsules/${shortId(c.id)}`"
                        class="flex items-start gap-3 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600"
                    >
                        <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                            <IPill class="w-4 h-4 text-zinc-400" />
                        </div>
                        <div class="min-w-0">
                            <div class="flex items-center gap-2 mb-1 min-w-0">
                                <CompoundBadge :compound="c.compound" />
                                <TruncatedTitle :title="c.title" class="text-sm text-(--text-h) font-medium" />
                            </div>
                            <p class="text-xs text-zinc-500 line-clamp-2" v-html="c.snippet" />
                            <p class="text-xs text-zinc-600 mt-1">{{ formatDate(c.updated_at) }}</p>
                        </div>
                    </RouterLink>
                </div>
                <div class="pt-3 flex justify-center">
                    <Paginator v-model:current-page="capsPage" :total="capsTotal" :page-size="capsPageSize" />
                </div>
            </div>

            <!-- Pills -->
            <div v-if="(scope === 'all' || scope === 'pills') && pillItems.length > 0">
                <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">
                    {{ $t("search.pills_heading") }} ({{ pillsTotal }})
                </h2>
                <div class="space-y-2">
                    <component
                        :is="p.bottle_id && p.prescription_id ? RouterLink : 'div'"
                        v-for="p in pillItems"
                        :key="p.id"
                        :to="p.bottle_id && p.prescription_id ? `/bottles/${shortId(p.bottle_id)}/prescriptions/${shortId(p.prescription_id)}/pills/${shortId(p.id)}` : undefined"
                        class="flex items-start gap-3 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600"
                    >
                        <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                            <IFileText class="w-4 h-4 text-zinc-400" />
                        </div>
                        <div class="min-w-0">
                            <div class="flex items-center gap-2 mb-1 min-w-0">
                                <CompoundBadge :compound="p.compound" />
                                <TruncatedTitle :title="p.title" class="text-sm text-(--text-h) font-medium" />
                            </div>
                            <p class="text-xs text-zinc-500 line-clamp-2" v-html="p.snippet" />
                            <p class="text-xs text-zinc-600 mt-1">{{ formatDate(p.updated_at) }}</p>
                        </div>
                    </component>
                </div>
                <div class="pt-3 flex justify-center">
                    <Paginator v-model:current-page="pillsPage" :total="pillsTotal" :page-size="pillsPageSize" />
                </div>
            </div>

            <EmptyState
                v-if="
                    (scope === 'all' && pillItems.length === 0 && capItems.length === 0) ||
                    (scope === 'pills' && pillItems.length === 0) ||
                    (scope === 'capsules' && capItems.length === 0)
                "
                padding="py-12"
                :text="$t('search.no_results', { query })"
            />
        </template>

        <div v-else class="text-center py-12 text-zinc-600 text-sm">
            {{ $t("search.hint") }}
        </div>
    </div>
</template>
