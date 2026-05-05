<script setup lang="ts">
import { ElTooltip } from 'element-plus'

defineOptions({ inheritAttrs: false })

withDefaults(defineProps<{
    title: string
    tag?: string
}>(), {
    tag: 'span',
})
</script>

<template>
    <ElTooltip
        :content="title"
        placement="top"
        effect="dark"
        :show-after="300"
        :hide-after="0"
    >
        <component :is="tag" class="truncate min-w-0 block" v-bind="$attrs">{{ title }}</component>
    </ElTooltip>
</template>
