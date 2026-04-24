<script setup lang="ts">
import ILockOpen from '~icons/lucide/lock-open'
import ILock from '~icons/lucide/lock'

defineProps<{ open: boolean }>()
</script>

<template>
    <span v-if="open" class="inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-md bg-green-500/10 text-green-500 border border-green-500/20 font-medium">
        <ILockOpen class="w-3 h-3" />
        {{ $t('common.status_open') }}
    </span>
    <span v-else class="inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-md bg-zinc-500/10 text-zinc-400 border border-zinc-700/50 font-medium">
        <ILock class="w-3 h-3" />
        {{ $t('common.status_closed') }}
    </span>
</template>

