<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { bottlesApi } from '@/core/infrastructure/repositories/BottlesRepository'
import { useActiveBottle } from '@/composables/useActiveBottle'
import type { Bottle } from '@/core/domain/types'
import { RouterLink } from 'vue-router'
import { useConfirm } from '@/composables/useConfirm'
import IBox from '~icons/lucide/box'
import IAlertTriangle from '~icons/lucide/alert-triangle'
import ITrash2 from '~icons/lucide/trash-2'

const { t } = useI18n()
const { confirm, prompt, alert } = useConfirm()
const { activeBottleId } = useActiveBottle()
const bottles = ref<Bottle[]>([])
const loading = ref(false)

async function load() {
    loading.value = true
    try { bottles.value = await bottlesApi.list() } finally { loading.value = false }
}

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
        await load()
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
        bottles.value = bottles.value.filter(x => x.reg_id !== b.reg_id)
    } catch { /* el error ya se muestra en el servidor */ }
}

onMounted(load)
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto space-y-5">
        <div class="flex items-center justify-between">
            <h1 class="text-2xl font-bold text-(--text-h)">{{ $t('nav.bottles') }}</h1>
        </div>

        <div v-if="loading" class="text-center py-16 text-zinc-500">{{ $t('common.loading') }}…</div>

        <div v-else-if="bottles.length === 0" class="text-center py-16 text-zinc-500">
            <p>{{ $t('bottles.empty') }}</p>
        </div>

        <div v-else class="space-y-2">
            <template v-for="b in bottles" :key="b.linked ? b.id : b.reg_id">

                <!-- Bottle vinculado (normal) -->
                <RouterLink
                    v-if="b.linked"
                    :to="`/bottles/${b.id}`"
                    class="flex items-center justify-between bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600 transition-colors"
                >
                    <div class="flex items-center gap-3 min-w-0">
                        <div class="relative shrink-0">
                            <div :class="b.id === activeBottleId ? 'bg-green-500/5 border border-green-500/50' : 'bg-(--accent-bg)'"
                                 class="w-9 h-9 rounded-lg flex items-center justify-center transition-colors">
                                <IBox :class="b.id === activeBottleId ? 'text-green-500' : 'text-zinc-400'" class="w-4 h-4" />
                            </div>
                            <span v-if="b.id === activeBottleId"
                                  class="absolute -top-1 -right-1 w-2.5 h-2.5 rounded-full bg-green-500 border-2 border-(--bg-surface)" />
                        </div>
                        <div class="min-w-0">
                            <div class="flex items-baseline gap-2">
                                <span class="text-(--text-h) font-medium">{{ b.display_name }}</span>
                                <span class="text-sm text-zinc-600 font-mono">{{ b.name }}</span>
                            </div>
                            <div class="flex items-center gap-1.5 mt-0.5 min-w-0">
                                <span class="text-[11px] text-zinc-600 border border-(--border) px-1.5 py-0 rounded shrink-0">{{ b.scope }}</span>
                                <span class="text-xs text-zinc-600 truncate">{{ b.directory }}</span>
                            </div>
                        </div>
                    </div>
                    <button
                        v-if="b.id !== activeBottleId"
                        class="text-xs text-zinc-400 hover:text-(--text-h) transition-colors ml-4 shrink-0"
                        @click.prevent="activeBottleId = b.id">
                        {{ $t('bottles.activate_btn') }}
                    </button>
                </RouterLink>

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
                            class="border border-(--border) px-2.5 py-1.5 rounded-lg text-xs text-zinc-400 hover:text-(--text-h) transition-colors"
                            @click="updateRegistration(b)">
                            {{ $t('bottles.update_registration_btn') }}
                        </button>
                        <button
                            class="flex items-center gap-1 border border-red-900/40 px-2.5 py-1.5 rounded-lg text-xs text-red-400 hover:text-red-300 transition-colors"
                            @click="deleteRegistration(b)">
                            <ITrash2 class="w-3 h-3" />
                            {{ $t('bottles.delete_registration_btn') }}
                        </button>
                    </div>
                </div>

            </template>
        </div>
    </div>
</template>
