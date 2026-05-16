<script setup lang="ts">
import { shortId } from "@/core/utils/id"
import { useClipboard } from "@vueuse/core"
import { computed } from "vue"
import { useI18n } from "vue-i18n"
import ICheck from "~icons/lucide/check"
import IHash from "~icons/lucide/hash"

const props = defineProps<{ id: string }>()

const { t } = useI18n()
const short = computed(() => shortId(props.id))
// `legacy: true` activa el fallback con document.execCommand cuando navigator.clipboard
// no está disponible (contextos sin HTTPS / IP local).
const { copy, copied } = useClipboard({ source: short, copiedDuring: 1500, legacy: true })
</script>

<template>
    <button
        type="button"
        class="flex items-center justify-center size-7 rounded-md border border-(--border) text-zinc-400 hover:text-(--text-h) cursor-pointer shrink-0"
        :title="copied ? t('common.copied') : `${t('common.copy_id')}: ${short}`"
        @click.stop.prevent="copy(short)"
    >
        <ICheck v-if="copied" class="w-3.5 h-3.5 text-green-500" />
        <IHash v-else class="w-3.5 h-3.5" />
    </button>
</template>
