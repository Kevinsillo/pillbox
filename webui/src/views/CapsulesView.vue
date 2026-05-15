<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfirm } from '@/composables/useConfirm'
import { usePoll } from '@/composables/usePoll'
import { capsulesApi } from '@/core/infrastructure/repositories/CapsulesRepository'
import { shortId } from '@/core/utils/id'
import type { CapsuleSummary } from '@/core/domain/types'
import CompoundBadge from '@/components/CompoundBadge.vue'
import TruncatedTitle from '@/components/TruncatedTitle.vue'
import { RouterLink } from 'vue-router'
import IPill from '~icons/lucide/pill'
import ITrash2 from '~icons/lucide/trash-2'

const { t } = useI18n()
const { confirm } = useConfirm()

const capsules = ref<CapsuleSummary[]>([])

const activeCapsules = computed(() => capsules.value.filter(c => c.deleted_at === null))
const archivedCapsules = computed(() => capsules.value.filter(c => c.deleted_at !== null))

let currentToken = 0

async function load() {
    const token = ++currentToken
    const next = await capsulesApi.list({})
    if (token !== currentToken) return
    capsules.value = next
}

const poll = usePoll(load, 5000)

async function archiveCapsule(c: CapsuleSummary) {
    try {
        await confirm(
            t('confirm.delete_capsule_msg', { title: c.title }),
            t('confirm.delete_capsule_title'),
            { confirmText: t('common.delete'), cancelText: t('common.cancel') }
        )
        await capsulesApi.delete(c.id)
        poll.restart()
    } catch { /* cancelled */ }
}

async function purgeCapsule(c: CapsuleSummary) {
    try {
        await confirm(
            t('confirm.purge_capsule_msg', { title: c.title }),
            t('confirm.purge_capsule_title'),
            { confirmText: t('common.delete_permanent'), cancelText: t('common.cancel'), waitSeconds: 3 }
        )
        await capsulesApi.purge(c.id)
        capsules.value = capsules.value.filter(x => x.id !== c.id)
    } catch { /* cancelled */ }
}
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <div class="flex items-center justify-between">
            <h1 class="text-2xl font-bold text-(--text-h)">{{ $t('nav.capsules') }}</h1>
        </div>

        <div v-if="!poll.loaded.value" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>

        <template v-else>
            <div
                class="transition-opacity duration-300"
                :class="poll.loaded.value ? 'opacity-100' : 'opacity-0'"
            >
                <div v-show="capsules.length === 0" class="text-center py-16 text-zinc-500">
                    {{ $t('capsules.empty') }}
                </div>

                <!-- Active capsules (siempre en DOM con v-show para preservar TransitionGroup) -->
                <TransitionGroup
                    v-show="capsules.length > 0"
                    name="list"
                    tag="div"
                    class="space-y-2 relative"
                >
                    <RouterLink
                        v-for="c in activeCapsules"
                        :key="`active-${c.id}`"
                        :to="`/capsules/${shortId(c.id)}`"
                        class="flex items-center justify-between gap-4 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
                    >
                        <div class="flex items-center gap-3 min-w-0">
                            <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                                <IPill class="w-4 h-4 text-zinc-400" />
                            </div>
                            <div class="space-y-1 min-w-0">
                                <div class="flex items-center gap-2 min-w-0">
                                    <CompoundBadge :compound="c.compound" />
                                    <TruncatedTitle :title="c.title" class="text-sm text-(--text-h) font-medium" />
                                </div>
                                <p class="text-xs text-zinc-600">{{ new Date(c.updated_at).toLocaleString() }}</p>
                            </div>
                        </div>
                        <div class="flex gap-2 shrink-0" @click.prevent>
                            <RouterLink
                                :to="`/capsules/${shortId(c.id)}`"
                                class="text-xs text-zinc-400 hover:text-(--text-h) border border-(--border) px-2.5 py-1.5 rounded-lg transition-colors"
                                @click.stop>
                                {{ $t('common.edit') }}
                            </RouterLink>
                            <button
                                class="flex items-center gap-1 text-xs text-red-400 hover:text-red-300 border border-red-900/40 px-2.5 py-1.5 rounded-lg transition-colors"
                                @click.stop="archiveCapsule(c)">
                                <ITrash2 class="w-3 h-3" />
                                {{ $t('common.delete') }}
                            </button>
                        </div>
                    </RouterLink>
                </TransitionGroup>

                <!-- Archived heading (siempre presente, oculto si no hay archivadas) -->
                <h3
                    v-show="archivedCapsules.length > 0"
                    class="text-xs font-semibold text-zinc-600 uppercase tracking-wider mt-4 mb-2"
                >
                    {{ $t('capsules.archived_heading') }} ({{ archivedCapsules.length }})
                </h3>

                <!-- Archived capsules -->
                <TransitionGroup
                    v-show="archivedCapsules.length > 0"
                    name="list"
                    tag="div"
                    class="space-y-2 relative"
                >
                    <div
                        v-for="c in archivedCapsules"
                        :key="`archived-${c.id}`"
                        class="relative opacity-50"
                    >
                        <RouterLink
                            :to="`/capsules/${shortId(c.id)}`"
                            class="flex items-center justify-between gap-4 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
                        >
                            <div class="flex items-center gap-3 min-w-0">
                                <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                                    <IPill class="w-4 h-4 text-zinc-400" />
                                </div>
                                <div class="space-y-1 min-w-0">
                                    <div class="flex items-center gap-2 min-w-0">
                                        <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-zinc-700/50 text-zinc-400">{{ $t('capsules.archived_badge') }}</span>
                                        <CompoundBadge :compound="c.compound" />
                                        <TruncatedTitle :title="c.title" class="text-sm text-(--text-h) font-medium" />
                                    </div>
                                    <p class="text-xs text-zinc-600">{{ new Date(c.updated_at).toLocaleString() }}</p>
                                </div>
                            </div>
                            <div class="flex gap-2 shrink-0" @click.prevent>
                                <button
                                    class="flex items-center gap-1 text-xs text-red-400 hover:text-red-300 border border-red-900/40 px-2.5 py-1.5 rounded-lg transition-colors"
                                    @click.stop="purgeCapsule(c)">
                                    <ITrash2 class="w-3 h-3" />
                                    {{ $t('common.delete_permanent') }}
                                </button>
                            </div>
                        </RouterLink>
                    </div>
                </TransitionGroup>
            </div>
        </template>
    </div>
</template>
