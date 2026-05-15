<script setup lang="ts">
import CompoundBadge from "@/components/CompoundBadge.vue"
import TruncatedTitle from "@/components/TruncatedTitle.vue"
import { useActiveBottle } from "@/composables/useActiveBottle"
import type { CapsuleSearchResult, Compound, PillSearchResult } from "@/core/domain/types"
import { capsulesApi } from "@/core/infrastructure/repositories/CapsulesRepository"
import { pillsApi } from "@/core/infrastructure/repositories/PillsRepository"
import { shortId } from "@/core/utils/id"
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
const pillResults = ref<PillSearchResult[]>([])
const capsuleResults = ref<CapsuleSearchResult[]>([])
const loading = ref(false)
const searched = ref(false)
const isFuzzy = ref(false)

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
        pillResults.value = []
        capsuleResults.value = []
        searched.value = false
        return
    }

    if (currentController) currentController.abort()
    currentController = new AbortController()

    loading.value = true
    searched.value = true

    const effectiveQuery = q
    const effectiveCompound = compound.value || undefined

    const runPills = (fuzzy: boolean) =>
        pillsApi.search({
            query: effectiveQuery,
            bottle_id: activeBottleId.value ?? undefined,
            compound: effectiveCompound,
            limit: 20,
            fuzzy,
        })
    const runCaps = (fuzzy: boolean) =>
        capsulesApi.search({
            query: effectiveQuery,
            compound: effectiveCompound,
            limit: 20,
            fuzzy,
        })

    try {
        // First pass: fuzzy=false
        let pills: PillSearchResult[] = []
        let caps: CapsuleSearchResult[] = []
        if (scope.value === "all") {
            ;[pills, caps] = await Promise.all([runPills(false), runCaps(false)])
        } else if (scope.value === "pills") {
            pills = await runPills(false)
        } else {
            caps = await runCaps(false)
        }

        let total = 0
        if (scope.value === "all") total = pills.length + caps.length
        else if (scope.value === "pills") total = pills.length
        else total = caps.length

        // Second pass: fuzzy=true if no results and query is long enough
        if (total === 0 && effectiveQuery.length > 4) {
            if (scope.value === "all") {
                ;[pills, caps] = await Promise.all([runPills(true), runCaps(true)])
            } else if (scope.value === "pills") {
                pills = await runPills(true)
            } else {
                caps = await runCaps(true)
            }
            const total2 =
                scope.value === "all"
                    ? pills.length + caps.length
                    : scope.value === "pills"
                      ? pills.length
                      : caps.length
            isFuzzy.value = total2 > 0
        }

        pillResults.value = pills
        capsuleResults.value = caps
    } finally {
        loading.value = false
    }
}

function triggerSearch() {
    clearTimeout(debounce)
    debounce = setTimeout(runSearch, 150)
}

function onInput() {
    isFuzzy.value = false
    triggerSearch()
}

function onScopeChange() {
    isFuzzy.value = false
    compound.value = ""
    loadCompounds()
    triggerSearch()
}

function onCompoundChange() {
    isFuzzy.value = false
    triggerSearch()
}

watch(activeBottleId, () => {
    loadCompounds()
    triggerSearch()
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

        <div v-if="loading" class="text-center py-12 text-zinc-500">{{ $t("search.searching") }}…</div>

        <template v-else-if="searched">
            <!-- Capsules -->
            <div v-if="(scope === 'all' || scope === 'capsules') && capsuleResults.length > 0">
                <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">
                    {{ $t("search.capsules_heading") }} ({{ capsuleResults.length }})
                </h2>
                <div class="space-y-2">
                    <RouterLink
                        v-for="c in capsuleResults"
                        :key="c.id"
                        :to="`/capsules/${shortId(c.id)}`"
                        class="flex items-start gap-3 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
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
                            <p class="text-xs text-zinc-600 mt-1">{{ new Date(c.updated_at).toLocaleDateString() }}</p>
                        </div>
                    </RouterLink>
                </div>
            </div>

            <!-- Pills -->
            <div v-if="(scope === 'all' || scope === 'pills') && pillResults.length > 0">
                <h2 class="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-3">
                    {{ $t("search.pills_heading") }} ({{ pillResults.length }})
                </h2>
                <div class="space-y-2">
                    <component
                        :is="p.bottle_id && p.prescription_id ? RouterLink : 'div'"
                        v-for="p in pillResults"
                        :key="p.id"
                        :to="p.bottle_id && p.prescription_id ? `/bottles/${shortId(p.bottle_id)}/prescriptions/${shortId(p.prescription_id)}/pills/${shortId(p.id)}` : undefined"
                        class="flex items-start gap-3 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
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
                            <p class="text-xs text-zinc-600 mt-1">{{ new Date(p.updated_at).toLocaleDateString() }}</p>
                        </div>
                    </component>
                </div>
            </div>

            <div
                v-if="
                    (scope === 'all' && pillResults.length === 0 && capsuleResults.length === 0) ||
                    (scope === 'pills' && pillResults.length === 0) ||
                    (scope === 'capsules' && capsuleResults.length === 0)
                "
                class="text-center py-12 text-zinc-500"
            >
                {{ $t("search.no_results", { query }) }}
            </div>
        </template>

        <div v-else class="text-center py-12 text-zinc-600 text-sm">
            {{ $t("search.hint") }}
        </div>
    </div>
</template>
