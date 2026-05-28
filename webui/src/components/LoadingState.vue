<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = withDefaults(
    defineProps<{
        /** Loading label (the trailing ellipsis is appended automatically). */
        text?: string
        /** Vertical padding utility, lets call-sites preserve their original spacing. */
        padding?: string
    }>(),
    { text: '', padding: 'py-16' },
)
</script>

<template>
    <div class="text-center text-zinc-500" :class="props.padding">{{ props.text || t('common.loading') }}…</div>
</template>
