<script setup lang="ts">
import { computed } from "vue"
import { RouterLink } from "vue-router"
import CompoundBadge from "@/components/CompoundBadge.vue"
import TruncatedTitle from "@/components/TruncatedTitle.vue"
import { shortId } from "@/core/utils/id"
import { formatDate } from "@/core/utils/date"
import IFileText from "~icons/lucide/file-text"
import IPill from "~icons/lucide/pill"

type Kind = "pill" | "capsule"

const props = defineProps<{
    kind: Kind
    id: string
    title: string
    compound: string
    snippet: string
    updatedAt: string
    // Pill-only:
    bottleId?: string | null
    prescriptionId?: string | null
}>()

const isPill = computed(() => props.kind === "pill")

// For pills, the row is a link only when both bottle_id and prescription_id are present.
const pillLinkable = computed(() => !!(props.bottleId && props.prescriptionId))

const pillTo = computed(() =>
    pillLinkable.value
        ? `/bottles/${shortId(props.bottleId as string)}/prescriptions/${shortId(props.prescriptionId as string)}/pills/${shortId(props.id)}`
        : undefined,
)

const capsuleTo = computed(() => `/capsules/${shortId(props.id)}`)
</script>

<template>
    <RouterLink
        v-if="!isPill"
        :to="capsuleTo"
        class="flex items-start gap-3 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600"
    >
        <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
            <IPill class="w-4 h-4 text-zinc-400" />
        </div>
        <div class="min-w-0">
            <div class="flex items-center gap-2 mb-1 min-w-0">
                <CompoundBadge :compound="compound" />
                <TruncatedTitle :title="title" class="text-sm text-(--text-h) font-medium" />
            </div>
            <p class="text-xs text-zinc-500 line-clamp-2" v-html="snippet" />
            <p class="text-xs text-zinc-600 mt-1">{{ formatDate(updatedAt) }}</p>
        </div>
    </RouterLink>

    <component
        v-else
        :is="pillLinkable ? RouterLink : 'div'"
        :to="pillTo"
        class="flex items-start gap-3 bg-(--bg-surface) border border-(--border) rounded-lg p-3 hover:border-zinc-600"
    >
        <div class="w-9 h-9 rounded-lg bg-(--accent-bg) flex items-center justify-center shrink-0">
            <IFileText class="w-4 h-4 text-zinc-400" />
        </div>
        <div class="min-w-0">
            <div class="flex items-center gap-2 mb-1 min-w-0">
                <CompoundBadge :compound="compound" />
                <TruncatedTitle :title="title" class="text-sm text-(--text-h) font-medium" />
            </div>
            <p class="text-xs text-zinc-500 line-clamp-2" v-html="snippet" />
            <p class="text-xs text-zinc-600 mt-1">{{ formatDate(updatedAt) }}</p>
        </div>
    </component>
</template>
