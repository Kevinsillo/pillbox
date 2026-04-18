<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessageBox } from 'element-plus'
import { capsulesApi } from '@/api/capsules'
import type { Capsule, CapsuleCompound } from '@/api/types'
import CompoundBadge from '@/components/CompoundBadge.vue'
import { marked } from 'marked'

const { t } = useI18n()
const props = defineProps<{ id: string }>()
const router = useRouter()

const capsule = ref<Capsule | null>(null)
const loading = ref(false)
const editing = ref(false)
const saving = ref(false)
const formError = ref<string | null>(null)

const CAPSULE_COMPOUNDS: CapsuleCompound[] = ['convention', 'workflow', 'environment', 'context', 'goal', 'feedback', 'manual']

const form = ref({ title: '', content: '', compound: 'convention' as CapsuleCompound })

async function load() {
    loading.value = true
    try {
        capsule.value = await capsulesApi.get(Number(props.id))
    } finally {
        loading.value = false }
}

onMounted(load)

function startEdit() {
    if (!capsule.value) return
    form.value = { title: capsule.value.title, content: capsule.value.content, compound: capsule.value.compound }
    editing.value = true
}

async function save() {
    saving.value = true
    formError.value = null
    try {
        capsule.value = await capsulesApi.update(Number(props.id), form.value)
        editing.value = false
    } catch (e: unknown) {
        formError.value = e instanceof Error ? e.message : 'Error'
    } finally {
        saving.value = false
    }
}

async function deleteCapsule() {
    try {
        await ElMessageBox.confirm(
            t('confirm.delete_capsule_msg', { title: capsule.value?.title }),
            t('confirm.delete_capsule_title'),
            {
                confirmButtonText: t('common.delete'),
                cancelButtonText: t('common.cancel'),
                type: 'warning',
            }
        )
        await capsulesApi.delete(Number(props.id))
        router.push('/capsules')
    } catch { /* cancelled */ }
}

const renderedContent = computed(() => capsule.value ? marked.parse(capsule.value.content) as string : '')
</script>

<template>
    <div class="p-6 max-w-3xl mx-auto space-y-5">
        <button class="text-xs text-zinc-500 hover:text-zinc-300 transition-colors" @click="router.back()">← {{ $t('capsule_detail.back') }}</button>

        <div v-if="loading" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>

        <template v-else-if="capsule">
            <!-- View mode -->
            <template v-if="!editing">
                <div class="flex items-start justify-between gap-4">
                    <div>
                        <CompoundBadge :compound="capsule.compound" />
                        <h1 class="text-xl font-bold text-(--text-h) mt-2">{{ capsule.title }}</h1>
                        <p class="text-xs text-zinc-500 mt-0.5">
                            {{ $t('capsule_detail.updated_at') }} {{ new Date(capsule.updated_at).toLocaleString() }}
                        </p>
                    </div>
                    <div class="flex gap-2 shrink-0">
                        <button
                            class="bg-(--accent-bg) hover:bg-zinc-600 text-(--text-h) text-sm px-3 py-2 rounded-lg transition-colors"
                            @click="startEdit">
                            {{ $t('common.edit') }}
                        </button>
                        <button
                            class="text-sm text-red-400 hover:text-red-300 border border-red-900/40 px-3 py-2 rounded-lg transition-colors"
                            @click="deleteCapsule">
                            {{ $t('common.delete') }}
                        </button>
                    </div>
                </div>

                <div class="bg-(--bg-surface) border border-(--border) rounded-lg p-5">
                    <div class="markdown" v-html="renderedContent" />
                </div>
            </template>

            <!-- Edit mode -->
            <template v-else>
                <h1 class="text-xl font-bold text-(--text-h)">{{ $t('capsule_detail.edit_heading') }}</h1>
                <form class="space-y-3" @submit.prevent="save">
                    <div>
                        <label class="block text-xs text-zinc-400 mb-1">{{ $t('common.compound') }}</label>
                        <select v-model="form.compound"
                                class="w-full bg-(--bg-surface) border border-(--border) rounded-md px-3 py-2 text-sm text-(--text-h) focus:outline-none focus:border-zinc-500">
                            <option v-for="c in CAPSULE_COMPOUNDS" :key="c" :value="c">{{ c }}</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-xs text-zinc-400 mb-1">{{ $t('common.title') }}</label>
                        <input v-model="form.title" required maxlength="255"
                               class="w-full bg-(--bg-surface) border border-(--border) rounded-md px-3 py-2 text-sm text-(--text-h) focus:outline-none focus:border-zinc-500" />
                    </div>
                    <div>
                        <label class="block text-xs text-zinc-400 mb-1">{{ $t('common.content_md') }}</label>
                        <textarea v-model="form.content" required rows="10" maxlength="5000"
                                  class="w-full bg-(--bg-surface) border border-(--border) rounded-md px-3 py-2 text-sm text-(--text-h) focus:outline-none focus:border-zinc-500 resize-y font-mono" />
                    </div>
                    <p v-if="formError" class="text-red-400 text-xs">{{ formError }}</p>
                    <div class="flex gap-2">
                        <button type="submit" :disabled="saving"
                                class="bg-(--accent-bg) hover:bg-zinc-600 disabled:opacity-50 text-(--text-h) text-sm px-4 py-2 rounded-lg transition-colors">
                            {{ saving ? ($t('common.saving') + '…') : $t('common.save') }}
                        </button>
                        <button type="button" @click="editing = false"
                                class="text-sm text-zinc-400 hover:text-(--text-h) px-3 py-2 transition-colors">
                            {{ $t('common.cancel') }}
                        </button>
                    </div>
                </form>
            </template>
        </template>
    </div>
</template>
