<script setup lang="ts">
import { ElAlert, ElInput, ElOption, ElSelect } from "element-plus"
import { pillsApi } from "@/core/infrastructure/repositories/PillsRepository"
import { prescriptionsApi } from "@/core/infrastructure/repositories/PrescriptionsRepository"
import type { Compound } from "@/core/domain/types"
import LoadingState from "@/components/LoadingState.vue"
import { onMounted, ref, computed } from "vue"
import { useI18n } from "vue-i18n"
import { useRouter } from "vue-router"
import IArrowLeft from "~icons/lucide/arrow-left"

const { t } = useI18n()
const props = defineProps<{ bottle_id: string; rx_id: string; pill_id: string }>()
const router = useRouter()

const loading = ref(false)
const saving = ref(false)
const error = ref<string | null>(null)

const compounds = ref<Compound[]>([])

const form = ref({ title: "", content: "", compound: "" })
const rxClosed = ref(false)
const locked = computed(() => rxClosed.value)

async function loadCompounds() {
    try {
        compounds.value = await pillsApi.getCompounds(props.bottle_id ?? undefined)
    } catch {
        compounds.value = []
    }
}

async function load() {
    loading.value = true
    try {
        const [pill, rx] = await Promise.all([
            pillsApi.get(props.pill_id, props.bottle_id, props.rx_id),
            prescriptionsApi.get(props.bottle_id, props.rx_id),
        ])
        form.value = { title: pill.title, content: pill.content, compound: pill.compound }
        rxClosed.value = !!rx.ended_at && !rx.deleted_at
    } finally {
        loading.value = false
    }
}

onMounted(() => {
    load()
    loadCompounds()
})

async function save() {
    saving.value = true
    error.value = null
    try {
        await pillsApi.update(props.pill_id, form.value, props.bottle_id, props.rx_id)
        router.back()
    } catch (e: unknown) {
        error.value = e instanceof Error ? e.message : t("common.error")
    } finally {
        saving.value = false
    }
}
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <button class="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-300" @click="router.back()">
            <IArrowLeft class="w-3 h-3" /> {{ $t("common.back") }}
        </button>

        <LoadingState v-if="loading" />

        <template v-else>
            <h1 class="text-xl font-bold text-(--text-h)">{{ $t("pill_edit.heading") }}</h1>

            <div v-if="locked" class="text-xs text-zinc-500 border border-(--border) bg-(--bg-surface) px-3 py-2 rounded-lg">
                {{ $t('pill_edit.closed_rx_hint') }}
            </div>

            <form class="space-y-3" @submit.prevent="save">
                <div>
                    <label class="block text-xs text-zinc-400 mb-1">{{ $t("common.compound") }}</label>
                    <el-select v-model="form.compound" filterable allow-create class="w-full">
                        <el-option v-for="c in compounds" :key="c.compound" :value="c.compound" :label="c.compound" />
                    </el-select>
                </div>
                <div>
                    <label class="block text-xs text-zinc-400 mb-1">{{ $t("common.title") }}</label>
                    <el-input v-model="form.title" maxlength="255" class="w-full" />
                </div>
                <div>
                    <label class="block text-xs text-zinc-400 mb-1">{{ $t("common.content_md") }}</label>
                    <el-input v-model="form.content" type="textarea" :rows="16" maxlength="5000" class="w-full font-mono" />
                </div>
                <el-alert
                    v-if="error"
                    :title="$t('common.error_saving')"
                    :description="error"
                    type="error"
                    show-icon
                    closable
                    @close="error = null"
                />
                <div class="flex gap-2">
                    <button
                        type="submit"
                        :disabled="saving || locked"
                        class="bg-(--accent-bg) hover:bg-zinc-600 text-(--text-h) text-sm px-4 py-2 rounded-lg disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {{ saving ? $t("common.saving") + "…" : $t("common.save") }}
                    </button>
                    <button
                        type="button"
                        @click="router.back()"
                        class="text-sm text-zinc-400 hover:text-(--text-h) px-3 py-2"
                    >
                        {{ $t("common.cancel") }}
                    </button>
                </div>
            </form>
        </template>
    </div>
</template>
