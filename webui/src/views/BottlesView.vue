<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { bottlesApi } from '@/core/infrastructure/repositories/BottlesRepository'
import { useActiveBottle } from '@/composables/useActiveBottle'
import type { Bottle } from '@/core/domain/types'
import { RouterLink } from 'vue-router'
import { ElMessageBox } from 'element-plus'
import IBox from '~icons/lucide/box'
import IAlertTriangle from '~icons/lucide/alert-triangle'

const { t } = useI18n()
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
        result = await ElMessageBox.prompt(t('bottles.update_registration_hint'), t('bottles.update_registration_btn'), {
            inputValue: b.directory + '/.pillbox/pillbox.db',
            inputPlaceholder: '/ruta/al/proyecto/.pillbox/pillbox.db',
            inputValidator: (v: string) => /\/.pillbox\/pillbox\.db$/.test(v) || t('bottles.update_registration_invalid'),
            confirmButtonText: t('common.save'),
            cancelButtonText: t('common.cancel'),
        })
    } catch { return }
    try {
        await bottlesApi.updateRegistration(b.reg_id, result.value)
        await load()
    } catch (e: unknown) {
        ElMessageBox.alert(e instanceof Error ? e.message : t('common.error'), { type: 'error' })
    }
}

async function deleteRegistration(b: Bottle) {
    if (!b.reg_id) return
    try {
        await ElMessageBox.confirm(t('bottles.confirm_delete_registration'), { type: 'warning', showClose: false })
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
                        <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
                            <IBox class="w-4 h-4 text-zinc-400" />
                        </div>
                        <div class="min-w-0">
                            <div class="flex items-center gap-2">
                                <span class="text-(--text-h) font-medium">{{ b.display_name }}</span>
                                <span class="text-xs text-zinc-600 font-mono">{{ b.name }}</span>
                                <span class="text-xs px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-500">{{ b.scope }}</span>
                                <span v-if="b.id === activeBottleId" class="text-xs text-green-500">● {{ $t('bottles.active_badge') }}</span>
                            </div>
                            <p class="text-xs text-zinc-600 mt-0.5 truncate">{{ b.directory }}</p>
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
                    class="flex items-center justify-between bg-(--bg-surface) border border-red-900/40 rounded-lg p-3"
                >
                    <div class="flex items-center gap-3 min-w-0">
                        <div class="w-9 h-9 rounded-lg bg-red-950/40 flex items-center justify-center shrink-0">
                            <IAlertTriangle class="w-4 h-4 text-red-500" />
                        </div>
                        <div class="min-w-0">
                            <div class="flex items-center gap-2">
                                <span class="text-(--text-h) font-medium">{{ b.display_name }}</span>
                                <span class="text-xs text-zinc-600 font-mono">{{ b.name }}</span>
                                <span class="text-xs px-1.5 py-0.5 rounded bg-red-950 text-red-500">{{ $t('bottles.unlinked_badge') }}</span>
                            </div>
                            <p class="text-xs text-red-800 mt-0.5 truncate" :title="b.directory">{{ b.directory }}</p>
                        </div>
                    </div>
                    <div class="flex items-center gap-3 ml-4 shrink-0">
                        <button
                            class="text-xs text-zinc-400 hover:text-(--text-h) transition-colors"
                            @click="updateRegistration(b)">
                            {{ $t('bottles.update_registration_btn') }}
                        </button>
                        <button
                            class="text-xs text-red-500 hover:text-red-400 transition-colors"
                            @click="deleteRegistration(b)">
                            {{ $t('bottles.delete_registration_btn') }}
                        </button>
                    </div>
                </div>

            </template>
        </div>
    </div>
</template>
