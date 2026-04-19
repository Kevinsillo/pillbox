<script setup lang="ts">
import type { PillCompound } from "@/core/domain/types"
import { pillsApi } from "@/core/infrastructure/repositories/PillsRepository"
import { onMounted, ref } from "vue"
import { useI18n } from "vue-i18n"
import { useRouter } from "vue-router"
import IArrowLeft from "~icons/lucide/arrow-left"

const { t } = useI18n()
const props = defineProps<{ id: string }>()
const router = useRouter()

const loading = ref(false)
const saving = ref(false)
const error = ref<string | null>(null)

const COMPOUNDS: PillCompound[] = [
    "decision",
    "architecture",
    "bugfix",
    "pattern",
    "discovery",
    "learning",
    "feedback",
    "prescription_summary",
    "manual",
]

const form = ref({ title: "", content: "", compound: "manual" as PillCompound })

async function load() {
    loading.value = true
    try {
        const pill = await pillsApi.get(Number(props.id))
        form.value = { title: pill.title, content: pill.content, compound: pill.compound as PillCompound }
    } finally {
        loading.value = false
    }
}

onMounted(load)

async function save() {
    saving.value = true
    error.value = null
    try {
        await pillsApi.update(Number(props.id), form.value)
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
        <button class="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-300 transition-colors" @click="router.back()">
            <IArrowLeft class="w-3 h-3" /> {{ $t("common.back") }}
        </button>

        <div v-if="loading" class="text-center py-16 text-zinc-500">{{ $t("common.loading") }}…</div>

        <template v-else>
            <h1 class="text-xl font-bold text-(--text-h)">{{ $t("pill_edit.heading") }}</h1>

            <form class="space-y-3" @submit.prevent="save">
                <div>
                    <label class="block text-xs text-zinc-400 mb-1">{{ $t("common.compound") }}</label>
                    <select
                        v-model="form.compound"
                        class="w-full bg-(--bg-surface) border border-(--border) rounded-lg px-3 py-2 text-sm text-(--text-h) focus:outline-none focus:border-zinc-500"
                    >
                        <option v-for="c in COMPOUNDS" :key="c" :value="c">{{ c }}</option>
                    </select>
                </div>
                <div>
                    <label class="block text-xs text-zinc-400 mb-1">{{ $t("common.title") }}</label>
                    <input
                        v-model="form.title"
                        required
                        maxlength="255"
                        class="w-full bg-(--bg-surface) border border-(--border) rounded-lg px-3 py-2 text-sm text-(--text-h) focus:outline-none focus:border-zinc-500"
                    />
                </div>
                <div>
                    <label class="block text-xs text-zinc-400 mb-1">{{ $t("common.content_md") }}</label>
                    <textarea
                        v-model="form.content"
                        required
                        rows="16"
                        maxlength="5000"
                        class="w-full bg-(--bg-surface) border border-(--border) rounded-lg px-3 py-2 text-sm text-(--text-h) focus:outline-none focus:border-zinc-500 resize-y font-mono"
                    />
                </div>
                <p v-if="error" class="text-red-400 text-xs">{{ error }}</p>
                <div class="flex gap-2">
                    <button
                        type="submit"
                        :disabled="saving"
                        class="bg-(--accent-bg) hover:bg-zinc-600 text-(--text-h) text-sm px-4 py-2 rounded-lg transition-colors disabled:opacity-50"
                    >
                        {{ saving ? $t("common.saving") + "…" : $t("common.save") }}
                    </button>
                    <button
                        type="button"
                        @click="router.back()"
                        class="text-sm text-zinc-400 hover:text-(--text-h) px-3 py-2 transition-colors"
                    >
                        {{ $t("common.cancel") }}
                    </button>
                </div>
            </form>
        </template>
    </div>
</template>
