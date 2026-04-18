<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'

useI18n()
</script>

<template>
    <div class="flex flex-col items-center justify-center h-screen text-zinc-500">
        <p class="text-6xl mb-4">{{ $t('not_found.code') }}</p>
        <p class="mb-4">{{ $t('not_found.message') }}</p>
        <RouterLink to="/" class="text-sm text-zinc-400 hover:text-(--text-h) underline">← {{ $t('not_found.back_link') }}</RouterLink>
    </div>
</template>
