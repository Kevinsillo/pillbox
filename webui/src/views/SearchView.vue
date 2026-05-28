<script setup lang="ts">
import Paginator from "@/components/Paginator.vue"
import LoadingState from "@/components/LoadingState.vue"
import EmptyState from "@/components/EmptyState.vue"
import SearchResultCard from "@/components/SearchResultCard.vue"
import { useSearch } from "@/composables/useSearch"
import { ElAlert, ElInput, ElOption, ElRadioButton, ElRadioGroup, ElSelect } from "element-plus"
import { onMounted } from "vue"
import { useI18n } from "vue-i18n"

const { t } = useI18n()

const {
    query,
    scope,
    compound,
    compounds,
    pillItems,
    pillsTotal,
    capItems,
    capsTotal,
    loading,
    searched,
    isFuzzy,
    pillsPage,
    pillsPageSize,
    capsPage,
    capsPageSize,
    loadCompounds,
    onInput,
    onScopeChange,
    onCompoundChange,
} = useSearch()

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
                    <SearchResultCard
                        v-for="c in capItems"
                        :key="c.id"
                        kind="capsule"
                        :id="c.id"
                        :title="c.title"
                        :compound="c.compound"
                        :snippet="c.snippet"
                        :updated-at="c.updated_at"
                    />
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
                    <SearchResultCard
                        v-for="p in pillItems"
                        :key="p.id"
                        kind="pill"
                        :id="p.id"
                        :title="p.title"
                        :compound="p.compound"
                        :snippet="p.snippet"
                        :updated-at="p.updated_at"
                        :bottle-id="p.bottle_id"
                        :prescription-id="p.prescription_id"
                    />
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
