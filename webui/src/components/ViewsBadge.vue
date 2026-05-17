<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import IEye from '~icons/lucide/eye'

defineProps<{ views: number }>()

const { t } = useI18n()
</script>

<template>
    <span
        class="inline-flex items-center gap-1 text-xs text-zinc-600"
        :aria-label="t('common.views_label', { count: views })"
    >
        <IEye class="w-3 h-3" />
        <span>{{ views }}</span>
    </span>
</template>
