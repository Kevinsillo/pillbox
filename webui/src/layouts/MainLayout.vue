<script setup lang="ts">
import { bottlesApi } from "@/api/bottles"
import type { Bottle } from "@/api/types"
import { useActiveBottle } from "@/composables/useActiveBottle"
import { useLocale } from "@/composables/useLocale"
import { onMounted, ref } from "vue"
import { RouterLink, RouterView, useRoute } from "vue-router"
import { useI18n } from "vue-i18n"
import ILayoutDashboard from "~icons/lucide/layout-dashboard"
import IBox from "~icons/lucide/box"
import IPill from "~icons/lucide/pill"
import ISearch from "~icons/lucide/search"

const { t } = useI18n()
const { availableLocales, currentLocale, setLocale } = useLocale()

const route = useRoute()
const { activeBottleId } = useActiveBottle()

const bottles = ref<Bottle[]>([])

onMounted(async () => {
    try {
        bottles.value = await bottlesApi.list()
        if (activeBottleId.value === null && bottles.value.length > 0) {
            activeBottleId.value = bottles.value[0].id
        }
    } catch {}
})

const activeBottle = () => bottles.value.find(b => b.id === activeBottleId.value)

const navItems = [
    { path: "/", label: "nav.dashboard", icon: ILayoutDashboard },
    { path: "/bottles", label: "nav.bottles", icon: IBox },
    { path: "/capsules", label: "nav.capsules", icon: IPill },
    { path: "/search", label: "nav.search", icon: ISearch },
]

const isActive = (path: string) => {
    if (path === "/") return route.path === "/"
    return route.path.startsWith(path)
}
</script>

<template>
    <div class="flex h-screen overflow-hidden bg-(--bg)">
        <!-- Sidebar -->
        <aside class="w-56 shrink-0 border-r border-(--border) flex flex-col">
            <div class="px-4 py-4 border-b border-(--border) flex items-center gap-2.5">
                <img src="/logo.svg" alt="Pillbox" class="w-7 h-7" />
                <span class="text-(--text-h) font-bold tracking-wide text-sm">Pillbox</span>
            </div>

            <!-- Bottle selector -->
            <div class="px-3 py-3 border-b border-(--border)">
                <p class="text-xs text-zinc-600 mb-1 uppercase tracking-wider">{{ $t('sidebar.active_bottle_label') }}</p>
                <select
                    v-model="activeBottleId"
                    class="w-full bg-(--accent-bg) border border-(--border) rounded-md text-xs text-(--text-h) px-2 py-1.5 focus:outline-none focus:border-zinc-500"
                >
                    <option v-if="bottles.length === 0" :value="null" disabled>{{ $t('sidebar.no_bottles') }}</option>
                    <option v-for="b in bottles" :key="b.id" :value="b.id">{{ b.display_name }}</option>
                </select>
                <p v-if="activeBottle()" class="text-xs text-zinc-600 mt-1 truncate" :title="activeBottle()!.directory">
                    {{ activeBottle()!.directory }}
                </p>
            </div>

            <!-- Nav -->
            <nav class="flex-1 px-2 py-3 space-y-0.5">
                <RouterLink
                    v-for="item in navItems"
                    :key="item.path"
                    :to="item.path"
                    :class="[
                        'flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors',
                        isActive(item.path)
                            ? 'bg-(--accent-bg) text-(--text-h)'
                            : 'text-(--text) hover:text-(--text-h) hover:bg-(--accent-bg)/50',
                    ]"
                >
                    <component :is="item.icon" class="w-4 h-4 shrink-0" />
                    {{ t(item.label) }}
                </RouterLink>
            </nav>

            <div class="px-4 py-3 border-t border-(--border) flex items-center justify-between">
                <span class="text-xs text-zinc-600">{{ $t('footer.version') }}</span>
                <div class="flex gap-1">
                    <button
                        v-for="loc in availableLocales"
                        :key="loc.code"
                        @click="setLocale(loc.code)"
                        :title="loc.name"
                        :class="[
                            'w-6 h-6 rounded flex items-center justify-center transition-colors',
                            currentLocale === loc.code ? 'bg-(--accent-bg)' : 'opacity-40 hover:opacity-100'
                        ]"
                    >
                        <component :is="loc.flag" class="w-4 h-3" />
                    </button>
                </div>
            </div>
        </aside>

        <!-- Main content -->
        <main class="flex-1 overflow-y-auto">
            <RouterView />
        </main>
    </div>
</template>
