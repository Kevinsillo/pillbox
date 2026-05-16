<script setup lang="ts">
import { ElPagination } from 'element-plus'
import { computed } from 'vue'

const props = defineProps<{
    currentPage: number
    total: number
    pageSize: number
}>()

const emit = defineEmits<{
    (e: 'update:currentPage', value: number): void
}>()

const visible = computed(() => props.total > props.pageSize)

function onChange(page: number) {
    emit('update:currentPage', page)
}
</script>

<template>
    <el-pagination
        v-if="visible"
        :current-page="currentPage"
        :page-size="pageSize"
        :total="total"
        layout="prev, pager, next, ->, total"
        background
        @current-change="onChange"
    >
        <template #total>
            <span class="text-zinc-500 text-sm">{{ $t('pagination.total', { total }) }}</span>
        </template>
    </el-pagination>
</template>
