<script setup lang="ts">
import CompoundBadge from "@/components/CompoundBadge.vue"
import CopyableId from "@/components/CopyableId.vue"
import HeaderMenu from "@/components/HeaderMenu.vue"
import ViewsBadge from "@/components/ViewsBadge.vue"
import LoadingState from "@/components/LoadingState.vue"
import type { Capsule } from "@/core/domain/types"
import { capsulesApi } from "@/core/infrastructure/repositories/CapsulesRepository"
import { formatDateTime } from "@/core/utils/date"
import { useConfirm } from "@/composables/useConfirm"
import { useMarkdown } from "@/composables/useMarkdown"
import { usePoll } from "@/composables/usePoll"
import { computed, ref, watchEffect } from "vue"
import { useI18n } from "vue-i18n"
import { useRouter } from "vue-router"
import IArrowLeft from "~icons/lucide/arrow-left"
import IPill from "~icons/lucide/pill"
import ITrash2 from "~icons/lucide/trash-2"

const { t } = useI18n()
const { confirm } = useConfirm()
const props = defineProps<{ id: string }>()
const router = useRouter()

const capsule = ref<Capsule | null>(null)

const isArchived = computed(() => !!capsule.value?.deleted_at)

async function load() {
    capsule.value = await capsulesApi.get(props.id)
}

const poll = usePoll(load, 5000)

function goToEdit() {
    router.push(`/capsules/${props.id}/edit`)
}

async function deleteCapsule() {
    try {
        await confirm(
            t("confirm.delete_capsule_msg", { title: capsule.value?.title }),
            t("confirm.delete_capsule_title"),
            { confirmText: t("common.delete"), cancelText: t("common.cancel") }
        )
        await capsulesApi.delete(props.id)
        router.push("/capsules")
    } catch {
        /* cancelled */
    }
}

async function purgeCapsule() {
    try {
        await confirm(
            t("confirm.purge_capsule_msg", { title: capsule.value?.title }),
            t("confirm.purge_capsule_title"),
            { confirmText: t("common.delete_permanent"), cancelText: t("common.cancel") }
        )
        await capsulesApi.purge(props.id)
        router.push("/capsules")
    } catch {
        /* cancelled */
    }
}

const { parse } = useMarkdown()
const renderedContent = ref("")
watchEffect(async () => {
    renderedContent.value = capsule.value ? await parse(capsule.value.content) : ""
})
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <button class="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-300" @click="router.back()"
            ><IArrowLeft class="w-3 h-3" /> {{ $t("capsule_detail.back") }}</button
        >

        <LoadingState v-if="!poll.loaded.value" />

        <template v-else-if="capsule">
            <div class="space-y-3">
                <div class="flex items-start justify-between gap-3">
                    <div class="flex gap-3 min-w-0">
                        <div
                            class="min-h-full w-20 rounded-lg bg-(--bg-surface) border border-(--border) flex items-center justify-center shrink-0"
                        >
                            <IPill class="size-8 text-zinc-400" />
                        </div>
                        <div class="min-w-0">
                            <div class="flex items-center gap-2 mb-1">
                                <CompoundBadge :compound="capsule.compound" />
                                <span v-if="isArchived" class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-zinc-700/50 text-zinc-400">{{ $t('capsule_detail.archived_badge') }}</span>
                            </div>
                            <div class="flex items-start gap-2 mt-2">
                                <CopyableId :id="capsule.id" />
                                <h1 class="text-xl font-bold text-(--text-h)">{{ capsule.title }}</h1>
                            </div>
                            <p class="text-xs text-zinc-500 mt-0.5 flex items-center gap-2 flex-wrap">
                                <ViewsBadge :views="capsule.views" />
                                <span>{{ $t("capsule_detail.updated_at") }} {{ formatDateTime(capsule.updated_at) }}</span>
                            </p>
                        </div>
                    </div>
                    <HeaderMenu entity="capsule" :capsule-id="props.id" />
                </div>
                <div class="flex gap-2">
                    <template v-if="!isArchived">
                        <button
                            class="bg-(--accent-bg) hover:bg-zinc-600 text-(--text-h) text-sm px-3 py-2 rounded-lg"
                            @click="goToEdit"
                        >
                            {{ $t("common.edit") }}
                        </button>
                        <button
                            class="flex items-center gap-1.5 text-sm btn-danger px-3 py-2"
                            @click="deleteCapsule"
                        >
                            <ITrash2 class="w-3.5 h-3.5" />
                            {{ $t("common.delete") }}
                        </button>
                    </template>
                    <button
                        v-if="isArchived"
                        class="flex items-center gap-1.5 text-sm btn-danger px-3 py-2"
                        @click="purgeCapsule"
                    >
                        <ITrash2 class="w-3.5 h-3.5" />
                        {{ $t("common.delete_permanent") }}
                    </button>
                </div>
            </div>

            <div class="bg-(--bg-surface) border border-(--border) rounded-lg p-5" :class="{ 'opacity-50': isArchived }">
                <div class="markdown" v-html="renderedContent" />
            </div>
        </template>
    </div>
</template>
