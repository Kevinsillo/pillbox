<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { bottlesApi } from '@/core/infrastructure/repositories/BottlesRepository'
import { useActiveBottle } from '@/composables/useActiveBottle'
import { usePoll } from '@/composables/usePoll'
import { usePaginatedList } from '@/composables/usePaginatedList'
import type { Bottle } from '@/core/domain/types'
import { useConfirm } from '@/composables/useConfirm'
import Paginator from '@/components/Paginator.vue'
import BottleCard from '@/components/BottleCard.vue'
import LoadingState from '@/components/LoadingState.vue'
import EmptyState from '@/components/EmptyState.vue'
import IAlertTriangle from '~icons/lucide/alert-triangle'
import ITrash2 from '~icons/lucide/trash-2'

const { t } = useI18n()
const { confirm, prompt, alert } = useConfirm()
const { activeBottleId } = useActiveBottle()

const { items, total, page, pageSize, refresh } = usePaginatedList<Bottle>({
    fetcher: (p) => bottlesApi.list(p),
})

const poll = usePoll(refresh, 5000)

async function updateRegistration(b: Bottle) {
    if (!b.reg_id) return
    let result
    try {
        result = await prompt(t('bottles.update_registration_hint'), t('bottles.update_registration_btn'), {
            inputValue: b.directory + '/.pillbox/pillbox.db',
            inputPlaceholder: '/ruta/al/proyecto/.pillbox/pillbox.db',
            inputValidator: (v: string) => /\/.pillbox\/pillbox\.db$/.test(v) || t('bottles.update_registration_invalid'),
            confirmText: t('common.save'),
            cancelText: t('common.cancel'),
        })
    } catch { return }
    try {
        await bottlesApi.updateRegistration(b.reg_id, result.value)
        poll.restart()
    } catch (e: unknown) {
        await alert(e instanceof Error ? e.message : t('common.error'))
    }
}

async function deleteRegistration(b: Bottle) {
    if (!b.reg_id) return
    try {
        await confirm(t('bottles.confirm_delete_registration'))
    } catch { return }
    try {
        await bottlesApi.deleteRegistration(b.reg_id)
        await refresh()
    } catch { /* el error ya se muestra en el servidor */ }
}
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <div class="flex items-center justify-between">
            <h1 class="text-2xl font-bold text-(--text-h)">{{ $t('nav.bottles') }}</h1>
        </div>

        <LoadingState v-if="!poll.loaded.value" />

        <EmptyState v-else-if="items.length === 0">
            <p>{{ $t('bottles.empty') }}</p>
        </EmptyState>

        <template v-else>
            <TransitionGroup
                name="list"
                tag="div"
                class="space-y-2 relative transition-opacity duration-300"
                :class="poll.loaded.value ? 'opacity-100' : 'opacity-0'"
            >
                <div v-for="b in items" :key="b.linked ? `b-${b.id}` : `r-${b.reg_id}`">
                    <!-- Bottle vinculado (normal) -->
                    <BottleCard
                        v-if="b.linked"
                        :bottle="b"
                        :active="b.id === activeBottleId"
                        @activate="activeBottleId = b.id"
                    />

                    <!-- Bottle desvinculado -->
                    <div
                        v-else
                        class="flex items-center justify-between bg-(--bg-surface) border border-(--chip-error-border) rounded-lg p-3"
                    >
                        <div class="flex items-center gap-3 min-w-0">
                            <div class="w-9 h-9 rounded-lg bg-(--icon-error-bg) flex items-center justify-center shrink-0">
                                <IAlertTriangle class="w-4 h-4 text-red-500" />
                            </div>
                            <div class="min-w-0">
                                <div class="flex items-center gap-2">
                                    <span class="text-(--text-h) font-medium">{{ b.display_name }}</span>
                                    <span class="text-xs text-zinc-600 font-mono">{{ b.name }}</span>
                                    <span class="text-xs px-1.5 py-0.5 rounded bg-(--chip-error-bg) text-(--chip-error-text)">{{ $t('bottles.unlinked_badge') }}</span>
                                </div>
                                <p class="text-xs text-(--text-error) mt-0.5 truncate" :title="b.directory">{{ b.directory }}</p>
                            </div>
                        </div>
                        <div class="flex items-center gap-3 ml-4 shrink-0">
                            <button
                                class="border border-(--border) px-2.5 py-1.5 rounded-lg text-xs text-zinc-400 hover:text-(--text-h)"
                                @click="updateRegistration(b)">
                                {{ $t('bottles.update_registration_btn') }}
                            </button>
                            <button
                                class="flex items-center gap-1 px-2.5 py-1.5 text-xs btn-danger"
                                @click="deleteRegistration(b)">
                                <ITrash2 class="w-3 h-3" />
                                {{ $t('bottles.delete_registration_btn') }}
                            </button>
                        </div>
                    </div>
                </div>
            </TransitionGroup>

            <div class="pt-4 flex justify-center">
                <Paginator v-model:current-page="page" :total="total" :page-size="pageSize" />
            </div>
        </template>
    </div>
</template>
