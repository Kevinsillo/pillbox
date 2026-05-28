<script setup lang="ts">
import CompoundBadge from "@/components/CompoundBadge.vue"
import CopyableId from "@/components/CopyableId.vue"
import HeaderMenu from "@/components/HeaderMenu.vue"
import ViewsBadge from "@/components/ViewsBadge.vue"
import LoadingState from "@/components/LoadingState.vue"
import type { Capsule, Compound } from "@/core/domain/types"
import { capsulesApi } from "@/core/infrastructure/repositories/CapsulesRepository"
import { formatDateTime } from "@/core/utils/date"
import { useConfirm } from "@/composables/useConfirm"
import { ElAlert, ElInput, ElOption, ElSelect } from "element-plus"
import { useMarkdown } from "@/composables/useMarkdown"
import { usePoll } from "@/composables/usePoll"
import { computed, onMounted, ref, watchEffect } from "vue"
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
const editing = ref(false)
const saving = ref(false)
const formError = ref<string | null>(null)

const compounds = ref<Compound[]>([])

const form = ref({ title: "", content: "", compound: "" })

const isArchived = computed(() => !!capsule.value?.deleted_at)

async function load() {
    capsule.value = await capsulesApi.get(props.id)
}

async function loadCompounds() {
    try {
        compounds.value = await capsulesApi.getCompounds()
    } catch {
        compounds.value = []
    }
}

const poll = usePoll(load, 5000)

onMounted(loadCompounds)

function startEdit() {
    if (!capsule.value) return
    form.value = { title: capsule.value.title, content: capsule.value.content, compound: capsule.value.compound }
    editing.value = true
    poll.stop()
}

async function save() {
    saving.value = true
    formError.value = null
    try {
        capsule.value = await capsulesApi.update(props.id, form.value)
        editing.value = false
        poll.restart()
    } catch (e: unknown) {
        formError.value = e instanceof Error ? e.message : "Error"
    } finally {
        saving.value = false
    }
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
            <!-- View mode -->
            <template v-if="!editing">
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
                                @click="startEdit"
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

            <!-- Edit mode -->
            <template v-else>
                <h1 class="text-xl font-bold text-(--text-h)">{{ $t("capsule_detail.edit_heading") }}</h1>
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
                        <el-input v-model="form.content" type="textarea" :rows="10" maxlength="5000" class="w-full font-mono" />
                    </div>
                    <el-alert
                        v-if="formError"
                        :title="$t('common.error_saving')"
                        :description="formError"
                        type="error"
                        show-icon
                        closable
                        @close="formError = null"
                    />
                    <div class="flex gap-2">
                        <button
                            type="submit"
                            :disabled="saving"
                            class="bg-(--accent-bg) hover:bg-zinc-600 disabled:opacity-50 text-(--text-h) text-sm px-4 py-2 rounded-lg"
                        >
                            {{ saving ? $t("common.saving") + "…" : $t("common.save") }}
                        </button>
                        <button
                            type="button"
                            @click="editing = false; poll.restart()"
                            class="text-sm text-zinc-400 hover:text-(--text-h) px-3 py-2"
                        >
                            {{ $t("common.cancel") }}
                        </button>
                    </div>
                </form>
            </template>
        </template>
    </div>
</template>
