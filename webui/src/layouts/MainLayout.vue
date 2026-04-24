<script setup lang="ts">
import ConfirmDialog from "@/components/ConfirmDialog.vue"
import { useActiveBottle } from "@/composables/useActiveBottle"
import { useLocale } from "@/composables/useLocale"
import { useTheme } from "@/composables/useTheme"
import type { Bottle } from "@/core/domain/types"
import { bottlesApi } from "@/core/infrastructure/repositories/BottlesRepository"
import { metaApi } from "@/core/infrastructure/repositories/MetaRepository"
import { ElDropdown, ElDropdownItem, ElDropdownMenu, ElOption, ElSelect } from "element-plus"
import { computed, onMounted, ref } from "vue"
import { useI18n } from "vue-i18n"
import { RouterLink, RouterView, useRoute } from "vue-router"
import IBox from "~icons/lucide/box"
import ILayoutDashboard from "~icons/lucide/layout-dashboard"
import IMoon from "~icons/lucide/moon"
import IPill from "~icons/lucide/pill"
import ISearch from "~icons/lucide/search"
import ISun from "~icons/lucide/sun"

const { t } = useI18n()
const { availableLocales, currentLocale, setLocale } = useLocale()
const { theme, toggleTheme } = useTheme()

const route = useRoute()
const { activeBottleId } = useActiveBottle()

const bottles = ref<Bottle[]>([])
const appVersion = ref<string>("")

const linkedBottles = computed(() => bottles.value.filter(b => b.linked))

onMounted(async () => {
    try {
        bottles.value = await bottlesApi.list()
        // Si el bottle activo ya no existe entre los vinculados, limpiar localStorage
        if (activeBottleId.value !== null && !linkedBottles.value.find(b => b.id === activeBottleId.value)) {
            activeBottleId.value = null
        }
        if (activeBottleId.value === null && linkedBottles.value.length > 0) {
            activeBottleId.value = linkedBottles.value[0].id
        }
    } catch {}
    try {
        const data = await metaApi.version()
        appVersion.value = `v${data.version}`
    } catch {}
})

const navItems = [
    { path: "/search", label: "nav.search", icon: ISearch },
    { path: "/", label: "nav.dashboard", icon: ILayoutDashboard },
    { path: "/bottles", label: "nav.bottles", icon: IBox },
    { path: "/capsules", label: "nav.capsules", icon: IPill },
]

const isActive = (path: string) => {
    if (path === "/") return route.path === "/"
    return route.path.startsWith(path)
}

const currentFlag = computed(() => availableLocales.value.find(l => l.code === currentLocale.value)?.flag)
</script>

<template>
    <div class="flex flex-col h-screen overflow-hidden bg-(--bg)">
        <!-- Navbar -->
        <header class="h-12 shrink-0 border-b border-(--border) flex items-center justify-between px-4">
            <div class="flex items-center gap-2.5">
                <svg viewBox="0 0 64 64" class="w-8 h-8" shape-rendering="geometricPrecision" aria-label="Pillbox">
                    <path
                        d="M16 14 L8 14 L8 50 L16 50"
                        fill="none"
                        :stroke="theme === 'dark' ? '#FAFAF7' : '#141414'"
                        stroke-width="4"
                        stroke-linecap="square"
                        stroke-linejoin="miter"
                    />
                    <path
                        d="M48 14 L56 14 L56 50 L48 50"
                        fill="none"
                        :stroke="theme === 'dark' ? '#FAFAF7' : '#141414'"
                        stroke-width="4"
                        stroke-linecap="square"
                        stroke-linejoin="miter"
                    />
                    <defs>
                        <clipPath id="logo-cap"><rect x="22" y="18" width="20" height="28" rx="10" /></clipPath>
                    </defs>
                    <g clip-path="url(#logo-cap)">
                        <rect x="22" y="18" width="20" height="14" :fill="theme === 'dark' ? '#FAFAF7' : '#141414'" />
                        <rect x="22" y="32" width="20" height="14" fill="#E8412A" />
                    </g>
                </svg>
                <span class="text-(--text-h) font-bold tracking-wide text-base">Pillbox</span>
            </div>

            <div class="flex items-center gap-1">
                <!-- Theme toggle -->
                <button
                    @click="toggleTheme"
                    class="flex items-center px-2 py-1 rounded-md hover:bg-(--accent-bg) transition-colors text-(--text)"
                    :aria-label="theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'"
                >
                    <ISun v-if="theme === 'dark'" class="w-4 h-4" />
                    <IMoon v-else class="w-4 h-4" />
                </button>

                <!-- Language dropdown -->
                <ElDropdown trigger="click" @command="setLocale">
                    <button class="flex items-center gap-1.5 px-2 py-1 rounded-md hover:bg-(--accent-bg) transition-colors">
                        <component :is="currentFlag" class="w-5 h-3.5" />
                    </button>
                    <template #dropdown>
                        <ElDropdownMenu>
                            <ElDropdownItem
                                v-for="loc in availableLocales"
                                :key="loc.code"
                                :command="loc.code"
                                :class="{ 'font-medium text-(--text-h)': loc.code === currentLocale }"
                            >
                                <span class="flex items-center gap-2">
                                    <component :is="loc.flag" class="w-5 h-3.5 shrink-0" />
                                    {{ loc.name }}
                                </span>
                            </ElDropdownItem>
                        </ElDropdownMenu>
                    </template>
                </ElDropdown>
            </div>
        </header>

        <div class="flex flex-1 overflow-hidden">
            <!-- Sidebar -->
            <aside class="w-56 shrink-0 border-r border-(--border) flex flex-col">
                <!-- Bottle selector -->
                <div class="px-3 py-3 border-b border-(--border)">
                    <p class="text-xs text-zinc-600 mb-1 uppercase tracking-wider">{{ $t("sidebar.active_bottle_label") }}</p>
                    <el-select
                        v-model="activeBottleId"
                        :placeholder="$t('sidebar.no_bottles')"
                        :disabled="linkedBottles.length === 0"
                        class="w-full"
                        size="small"
                    >
                        <el-option v-for="b in linkedBottles" :key="b.id" :value="b.id" :label="b.display_name" />
                    </el-select>
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

                <div class="px-4 py-3 border-t border-(--border)">
                    <span class="text-xs text-zinc-600">{{ appVersion }}</span>
                </div>
            </aside>

            <!-- Main content -->
            <main class="flex-1 overflow-y-auto">
                <RouterView />
            </main>
        </div>
    </div>
    <ConfirmDialog />
</template>
