<script setup lang="ts">
import { bottlesApi } from "@/core/infrastructure/repositories/BottlesRepository"
import { BarElement, CategoryScale, Chart as ChartJS, LinearScale, Tooltip } from "chart.js"
import { computed, ref, watch } from "vue"
import { Bar } from "vue-chartjs"
import { useI18n } from "vue-i18n"

ChartJS.register(CategoryScale, LinearScale, BarElement, Tooltip)

const { t } = useI18n()

const props = defineProps<{ bottleId: string }>()

type Period = "1d" | "1w" | "1m" | "1y"

const PERIODS: { key: Period; days: number; labelKey: string }[] = [
    { key: "1d", days: 1, labelKey: "dashboard.chart_period_1d" },
    { key: "1w", days: 7, labelKey: "dashboard.chart_period_1w" },
    { key: "1m", days: 30, labelKey: "dashboard.chart_period_1m" },
    { key: "1y", days: 365, labelKey: "dashboard.chart_period_1y" },
]

const activePeriod = ref<Period>("1m")
const labels = ref<string[]>([])
const counts = ref<number[]>([])
const loading = ref(false)

async function load(days: number) {
    loading.value = true
    try {
        const stats = await bottlesApi.stats(props.bottleId, days)
        labels.value = stats.pills_per_day.map(d => d.date)
        counts.value = stats.pills_per_day.map(d => d.count)
    } finally {
        loading.value = false
    }
}

watch(
    () => props.bottleId,
    () => {
        const period = PERIODS.find(p => p.key === activePeriod.value)!
        load(period.days)
    },
    { immediate: true },
)

function selectPeriod(p: Period) {
    activePeriod.value = p
    const period = PERIODS.find(x => x.key === p)!
    load(period.days)
}

const chartData = computed(() => ({
    labels: labels.value,
    datasets: [
        {
            data: counts.value,
            backgroundColor: "rgba(161,161,170,1)",
            hoverBackgroundColor: "rgba(212,212,216,1)",
            borderRadius: 3,
            borderSkipped: false,
        },
    ],
}))

const chartOptions = computed(() => ({
    responsive: true,
    maintainAspectRatio: false,
    animation: { duration: 200 },
    plugins: {
        legend: { display: false },
        tooltip: {
            callbacks: {
                title: (items: { label: string }[]) => items[0]?.label ?? "",
                label: (item: { raw: unknown }) => ` ${item.raw}`,
            },
            backgroundColor: "rgba(24,24,27,0.95)",
            titleColor: "#a1a1aa",
            bodyColor: "#fafaf7",
            borderColor: "#3f3f46",
            borderWidth: 1,
            padding: 8,
        },
    },
    scales: {
        x: {
            grid: { display: false },
            ticks: {
                color: "#71717a",
                font: { size: 10 },
                maxRotation: 0,
                autoSkip: true,
                maxTicksLimit: activePeriod.value === "1y" ? 12 : activePeriod.value === "1m" ? 6 : undefined,
            },
            border: { display: false },
        },
        y: {
            beginAtZero: true,
            grid: { color: "rgba(63,63,70,0.4)" },
            ticks: {
                color: "#71717a",
                font: { size: 10 },
                precision: 0,
            },
            border: { display: false },
        },
    },
}))
</script>

<template>
    <div class="bg-(--bg-surface) border border-(--border) rounded-lg p-4 flex flex-col gap-3">
        <div class="flex items-center justify-between">
            <span class="text-xs text-zinc-500">{{ $t("dashboard.chart_title") }}</span>
            <div class="flex gap-1">
                <button
                    v-for="p in PERIODS"
                    :key="p.key"
                    :class="activePeriod === p.key ? 'bg-(--accent-bg) text-(--text-h)' : 'text-zinc-500 hover:text-(--text-h)'"
                    class="px-2 py-0.5 rounded text-xs transition-colors"
                    @click="selectPeriod(p.key)"
                >
                    {{ t(p.labelKey) }}
                </button>
            </div>
        </div>
        <div class="h-28 relative">
            <div v-if="loading" class="absolute inset-0 flex items-center justify-center">
                <span class="text-xs text-zinc-600">…</span>
            </div>
            <Bar v-else :data="chartData" :options="chartOptions" />
        </div>
    </div>
</template>
