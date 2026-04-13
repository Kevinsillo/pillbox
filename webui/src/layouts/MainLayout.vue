<script setup lang="ts">
import Sidebar from "@/layouts/Sidebar.vue"
</script>

<template>
    <div class="flex h-screen overflow-hidden bg-(--bg)">
        <Sidebar class="w-64 shrink-0 border-r border-(--border) flex flex-col" />
        <main class="flex-1 overflow-auto p-6">
            <RouterView />
        </main>
    </div>
</template>
