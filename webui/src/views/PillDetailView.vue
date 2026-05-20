<script setup lang="ts">
import CompoundBadge from "@/components/CompoundBadge.vue"
import CopyableId from "@/components/CopyableId.vue"
import HeaderMenu from "@/components/HeaderMenu.vue"
import ViewsBadge from "@/components/ViewsBadge.vue"
import type { Pill } from "@/core/domain/types"
import { formatAuthor } from "@/core/domain/author"
import { pillsApi } from "@/core/infrastructure/repositories/PillsRepository"
import { prescriptionsApi } from "@/core/infrastructure/repositories/PrescriptionsRepository"
import type { Prescription } from "@/core/domain/types"
import { shortId } from "@/core/utils/id"
import { useConfirm } from "@/composables/useConfirm"
import { useMarkdown } from "@/composables/useMarkdown"
import { usePoll } from "@/composables/usePoll"
import { computed, ref, watchEffect } from "vue"
import { useI18n } from "vue-i18n"
import { useRouter } from "vue-router"
import IArrowLeft from "~icons/lucide/arrow-left"
import IEye from "~icons/lucide/eye"
import IFileText from "~icons/lucide/file-text"
import ILock from "~icons/lucide/lock"
import IFileCode from "~icons/lucide/file-code"
import ITrash2 from "~icons/lucide/trash-2"

const { t } = useI18n()
const { confirm } = useConfirm()
const props = defineProps<{ bottle_id: string; rx_id: string; pill_id: string }>()
const router = useRouter()

const pill = ref<Pill | null>(null)
const rx = ref<Prescription | null>(null)

const isArchived = computed(() => !!pill.value?.deleted_at)
const rxClosed = computed(() => !!rx.value?.ended_at && !rx.value?.deleted_at)
const canEdit = computed(() => !isArchived.value && !rxClosed.value)
const authorDisplay = computed(() =>
    pill.value ? formatAuthor(pill.value.author_name, pill.value.author_email) : null
)

async function load() {
    const [p, r] = await Promise.all([
        pillsApi.get(props.pill_id, props.bottle_id, props.rx_id),
        prescriptionsApi.get(props.bottle_id, props.rx_id),
    ])
    pill.value = p
    rx.value = r
}

const poll = usePoll(load, 5000)

async function archivePill() {
    if (!pill.value) return
    try {
        await confirm(
            t("confirm.delete_pill_msg", { title: pill.value.title }),
            t("confirm.delete_pill_title"),
            { confirmText: t("common.archive"), cancelText: t("common.cancel") }
        )
        await pillsApi.delete(props.pill_id, props.bottle_id, props.rx_id)
        poll.restart()
    } catch {
        /* cancelled */
    }
}

async function purgePill() {
    if (!pill.value) return
    try {
        await confirm(
            t("confirm.purge_pill_msg", { title: pill.value.title }),
            t("confirm.purge_pill_title"),
            { confirmText: t("common.delete_permanent"), cancelText: t("common.cancel"), waitSeconds: 3 }
        )
        await pillsApi.purge(props.pill_id, props.bottle_id, props.rx_id)
        router.back()
    } catch {
        /* cancelled */
    }
}

const { parse } = useMarkdown()
const renderedContent = ref("")
watchEffect(async () => {
    renderedContent.value = pill.value ? await parse(pill.value.content) : ""
})

const view = ref<"rendered" | "raw">("rendered")
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <button class="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-300" @click="router.back()">
            <IArrowLeft class="w-3 h-3" /> {{ $t("common.back") }}
        </button>

        <div v-if="!poll.loaded.value" class="text-center py-16 text-zinc-500">{{ $t("common.loading") }}…</div>

        <template v-else-if="pill">
            <div class="space-y-3">
                <div class="flex items-start justify-between gap-3">
                    <div class="flex gap-3 min-w-0">
                        <div
                            class="min-h-full w-20 rounded-lg bg-(--bg-surface) border border-(--border) flex items-center justify-center shrink-0"
                        >
                            <IFileText class="size-8 text-zinc-400" />
                        </div>
                        <div class="min-w-0">
                            <div class="flex items-center gap-2 flex-wrap">
                                <CompoundBadge :compound="pill.compound" />
                                <span v-if="pill.deleted_at" class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-(--badge-zinc-bg) text-(--badge-zinc-text)">{{ $t('pill_detail.archived_badge') }}</span>
                            </div>
                            <div class="flex items-start gap-2 mt-2">
                                <CopyableId :id="pill.id" />
                                <h1 class="text-xl font-bold text-(--text-h)">{{ pill.title }}</h1>
                            </div>
                            <p class="text-xs text-zinc-500 mt-0.5 flex items-center gap-2 flex-wrap">
                                <ViewsBadge :views="pill.views" />
                                <span>
                                    {{ $t("pill_detail.created_at") }} {{ new Date(pill.created_at).toLocaleString() }}
                                    <span v-if="pill.updated_at !== pill.created_at">
                                        · {{ $t("pill_detail.updated_at") }} {{ new Date(pill.updated_at).toLocaleString() }}
                                    </span>
                                </span>
                            </p>
                            <p v-if="authorDisplay" class="text-xs text-zinc-600 mt-0.5">
                                {{ $t('pill_detail.author') }}: {{ authorDisplay }}
                            </p>
                        </div>
                    </div>
                    <HeaderMenu entity="pill" :bottle-id="props.bottle_id" :rx-id="props.rx_id" :pill-id="props.pill_id" />
                </div>
                <div class="flex gap-2">
                    <template v-if="canEdit">
                        <button
                            class="bg-(--accent-bg) hover:bg-zinc-600 text-(--text-h) text-sm px-3 py-2 rounded-lg"
                            @click="router.push(`/bottles/${shortId(props.bottle_id)}/prescriptions/${shortId(props.rx_id)}/pills/${shortId(props.pill_id)}/edit`)"
                        >
                            {{ $t("common.edit") }}
                        </button>
                        <button
                            class="flex items-center gap-1.5 text-sm text-red-400 hover:text-red-300 border border-red-900/40 px-3 py-2 rounded-lg"
                            @click="archivePill"
                        >
                            <ITrash2 class="w-3.5 h-3.5" />
                            {{ $t("common.archive") }}
                        </button>
                    </template>
                    <button
                        v-if="isArchived"
                        class="flex items-center gap-1.5 text-sm text-red-400 hover:text-red-300 border border-red-900/40 px-3 py-2 rounded-lg"
                        @click="purgePill"
                    >
                        <ITrash2 class="w-3.5 h-3.5" />
                        {{ $t("common.delete_permanent") }}
                    </button>
                </div>
                <div v-if="rxClosed && !isArchived" class="flex items-start gap-2 text-sm text-(--callout-warn-text) border border-(--callout-warn-border) bg-(--callout-warn-bg) px-3 py-2 rounded-lg">
                    <ILock class="w-4 h-4 mt-0.5 shrink-0" />
                    <span>{{ $t('pill_detail.closed_rx_hint') }}</span>
                </div>
            </div>

            <div class="space-y-3">
                <div class="flex gap-1 border-b border-(--border)">
                    <button
                        type="button"
                        class="flex items-center gap-1.5 px-3 py-2 text-xs border-b-2 -mb-px transition-colors"
                        :class="view === 'rendered'
                            ? 'border-(--text-h) text-(--text-h)'
                            : 'border-transparent text-zinc-500 hover:text-zinc-300'"
                        @click="view = 'rendered'"
                    >
                        <IEye class="w-3.5 h-3.5" />
                        {{ $t("pill_detail.view_rendered") }}
                    </button>
                    <button
                        type="button"
                        class="flex items-center gap-1.5 px-3 py-2 text-xs border-b-2 -mb-px transition-colors"
                        :class="view === 'raw'
                            ? 'border-(--text-h) text-(--text-h)'
                            : 'border-transparent text-zinc-500 hover:text-zinc-300'"
                        @click="view = 'raw'"
                    >
                        <IFileCode class="w-3.5 h-3.5" />
                        {{ $t("pill_detail.view_raw") }}
                    </button>
                </div>
                <div class="bg-(--bg-surface) border border-(--border) rounded-lg p-5">
                    <div v-if="view === 'rendered'" class="markdown" v-html="renderedContent" />
                    <pre v-else class="text-sm font-mono text-(--text-h) whitespace-pre-wrap wrap-break-word">{{ pill.content }}</pre>
                </div>
            </div>
        </template>
    </div>
</template>
