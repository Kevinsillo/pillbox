<script setup lang="ts">
import { computed } from 'vue'
import { usePoll } from '@/composables/usePoll'
import { usePaginatedList } from '@/composables/usePaginatedList'
import { capsulesApi } from '@/core/infrastructure/repositories/CapsulesRepository'
import type { CapsuleSummary } from '@/core/domain/types'
import CapsuleCard from '@/components/CapsuleCard.vue'
import Paginator from '@/components/Paginator.vue'

const { items, total, page, pageSize, refresh } = usePaginatedList<CapsuleSummary>({
    fetcher: (p) => capsulesApi.list(p),
})

const activeCapsules = computed(() => items.value.filter(c => c.deleted_at === null))
const archivedCapsules = computed(() => items.value.filter(c => c.deleted_at !== null))

const poll = usePoll(refresh, 5000)
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
                <div v-show="items.length === 0" class="text-center py-16 text-zinc-500">
                    {{ $t('capsules.empty') }}
                </div>

                <!-- Active capsules -->
                <TransitionGroup
                    v-show="items.length > 0"
                    name="list"
                    tag="div"
                    class="space-y-2 relative"
                >
                    <CapsuleCard
                        v-for="c in activeCapsules"
                        :key="`active-${c.id}`"
                        :capsule="c"
                    />
                </TransitionGroup>

                <!-- Archived heading -->
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
                    <CapsuleCard
                        v-for="c in archivedCapsules"
                        :key="`archived-${c.id}`"
                        :capsule="c"
                        :archived="true"
                    />
                </TransitionGroup>

                <div v-if="items.length > 0" class="pt-4 flex justify-center">
                    <Paginator v-model:current-page="page" :total="total" :page-size="pageSize" />
                </div>
            </div>
        </template>
    </div>
</template>
