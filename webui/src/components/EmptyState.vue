<script setup lang="ts">
const props = withDefaults(
    defineProps<{
        /** Simple empty-state message. Ignored when the default slot is used. */
        text?: string
        /** Vertical padding utility, lets call-sites preserve their original spacing. */
        padding?: string
        /** Text color utility (some call-sites use a dimmer tone). */
        textClass?: string
    }>(),
    { text: '', padding: 'py-16', textClass: 'text-zinc-500' },
)
</script>

<template>
    <div class="text-center" :class="[props.padding, props.textClass]">
        <slot>{{ props.text }}</slot>
    </div>
</template>
