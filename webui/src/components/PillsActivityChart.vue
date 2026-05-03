<script setup lang="ts">
import type { BottleStats } from "@/core/domain/types"
import { BarElement, CategoryScale, Chart as ChartJS, LinearScale, Tooltip } from "chart.js"
import { computed } from "vue"
import { Bar } from "vue-chartjs"
import { useI18n } from "vue-i18n"

ChartJS.register(CategoryScale, LinearScale, BarElement, Tooltip)

const { t } = useI18n()

type Period = "1d" | "1w" | "1m" | "1y"

const PERIODS: { key: Period; days: number; labelKey: string }[] = [
    { key: "1d", days: 1, labelKey: "dashboard.chart_period_1d" },
    { key: "1w", days: 7, labelKey: "dashboard.chart_period_1w" },
    { key: "1m", days: 30, labelKey: "dashboard.chart_period_1m" },
    { key: "1y", days: 365, labelKey: "dashboard.chart_period_1y" },
]

const props = withDefaults(
    defineProps<{ stats: BottleStats | null; activePeriod?: Period }>(),
    { activePeriod: "1m" },
)

const emit = defineEmits<{ 'period-change': [days: number, key: Period] }>()

const labels = computed<string[]>(() => props.stats?.pills_per_day.map(d => d.date) ?? [])
const counts = computed<number[]>(() => props.stats?.pills_per_day.map(d => d.count) ?? [])

function selectPeriod(p: Period) {
    if (p === props.activePeriod) return
    const period = PERIODS.find(x => x.key === p)!
    emit('period-change', period.days, period.key)
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
                maxTicksLimit: props.activePeriod === "1y" ? 12 : props.activePeriod === "1m" ? 6 : undefined,
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

const hasData = computed(() => props.stats !== null)
</script>

<template>
    <div class="bg-(--bg-surface) border border-(--border) rounded-lg p-4 flex flex-col gap-3">
        <div class="flex items-center justify-between">
            <span class="text-xs text-zinc-500">{{ $t("dashboard.chart_title") }}</span>
            <div class="flex gap-1">
                <button
                    v-for="p in PERIODS"
                    :key="p.key"
                    :class="props.activePeriod === p.key ? 'bg-(--accent-bg) text-(--text-h)' : 'text-zinc-500 hover:text-(--text-h)'"
                    class="px-2 py-0.5 rounded text-xs transition-colors"
                    @click="selectPeriod(p.key)"
                >
                    {{ t(p.labelKey) }}
                </button>
            </div>
        </div>
        <div class="h-28 relative">
            <div v-if="!hasData" class="absolute inset-0 flex items-center justify-center">
                <span class="text-xs text-zinc-600">…</span>
            </div>
            <Bar v-else :data="chartData" :options="chartOptions" />
        </div>
    </div>
</template>
