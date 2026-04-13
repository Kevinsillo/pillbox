<script setup lang="ts">
import { onMounted, watch, computed } from "vue"
import { useRouter } from "vue-router"
import { useEngramStore } from "@/stores/engramStore"
import { useUiStore } from "@/stores/uiStore"
import type { Session } from "@/types/engram"
</script>

<template>
    <aside class="w-64 shrink-0 border-r border-(--border) flex flex-col h-full bg-(--bg)"></aside>
</template>
