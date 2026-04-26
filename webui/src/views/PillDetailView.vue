<script setup lang="ts">
import CompoundBadge from "@/components/CompoundBadge.vue"
import type { Pill } from "@/core/domain/types"
import { pillsApi } from "@/core/infrastructure/repositories/PillsRepository"
import { useConfirm } from "@/composables/useConfirm"
import { marked } from "marked"
import { computed, onMounted, ref } from "vue"
import { useI18n } from "vue-i18n"
import { useRouter } from "vue-router"
import IArrowLeft from "~icons/lucide/arrow-left"
import IFileText from "~icons/lucide/file-text"
import ITrash2 from "~icons/lucide/trash-2"

const { t } = useI18n()
const { confirmDual } = useConfirm()
const props = defineProps<{ bottle_id: string; rx_id: string; pill_id: string }>()
const router = useRouter()

const pill = ref<Pill | null>(null)
const loading = ref(false)

async function load() {
    loading.value = true
    try {
        pill.value = await pillsApi.get(Number(props.pill_id), props.bottle_id, props.rx_id)
    } finally {
        loading.value = false
    }
}

onMounted(load)

async function deletePill() {
    if (!pill.value) return
    try {
        const mode = await confirmDual(
            t("confirm.delete_pill_msg", { title: pill.value.title }),
            t("confirm.delete_pill_title"),
            { softText: t("common.archive"), hardText: t("common.delete_permanent"), cancelText: t("common.cancel") }
        )
        if (mode === "soft") {
            await pillsApi.delete(Number(props.pill_id), props.bottle_id, props.rx_id)
        } else {
            await pillsApi.purge(Number(props.pill_id), props.bottle_id, props.rx_id)
        }
        router.back()
    } catch {
        /* cancelled */
    }
}

const renderedContent = computed(() => (pill.value ? (marked.parse(pill.value.content) as string) : ""))
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <button class="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-300 transition-colors" @click="router.back()">
            <IArrowLeft class="w-3 h-3" /> {{ $t("common.back") }}
        </button>

        <div v-if="loading" class="text-center py-16 text-zinc-500">{{ $t("common.loading") }}…</div>

        <template v-else-if="pill">
            <div class="space-y-3">
                <div class="flex gap-3">
                    <div
                        class="min-h-full w-20 rounded-lg bg-(--bg-surface) border border-(--border) flex items-center justify-center shrink-0"
                    >
                        <IFileText class="size-8 text-zinc-400" />
                    </div>
                    <div>
                        <CompoundBadge :compound="pill.compound" />
                        <h1 class="text-xl font-bold text-(--text-h) mt-2">{{ pill.title }}</h1>
                        <p class="text-xs text-zinc-500 mt-0.5">
                            {{ $t("pill_detail.created_at") }} {{ new Date(pill.created_at).toLocaleString() }}
                            <span v-if="pill.updated_at !== pill.created_at">
                                · {{ $t("pill_detail.updated_at") }} {{ new Date(pill.updated_at).toLocaleString() }}
                            </span>
                        </p>
                        <p v-if="pill.author_name" class="text-xs text-zinc-600 mt-0.5">{{ pill.author_name }}</p>
                    </div>
                </div>
                <div class="flex gap-2">
                    <button
                        class="bg-(--accent-bg) hover:bg-zinc-600 text-(--text-h) text-sm px-3 py-2 rounded-lg transition-colors"
                        @click="router.push(`/bottles/${props.bottle_id}/prescriptions/${props.rx_id}/pills/${props.pill_id}/edit`)"
                    >
                        {{ $t("common.edit") }}
                    </button>
                    <button
                        class="flex items-center gap-1.5 text-sm text-red-400 hover:text-red-300 border border-red-900/40 px-3 py-2 rounded-lg transition-colors"
                        @click="deletePill"
                    >
                        <ITrash2 class="w-3.5 h-3.5" />
                        {{ $t("common.delete") }}
                    </button>
                </div>
            </div>

            <div class="bg-(--bg-surface) border border-(--border) rounded-lg p-5">
                <div class="markdown" v-html="renderedContent" />
            </div>
        </template>
    </div>
</template>
