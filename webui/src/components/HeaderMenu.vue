<script setup lang="ts">
import { shortId } from '@/core/utils/id'
import { useClipboard } from '@vueuse/core'
import { ElDropdown, ElDropdownItem, ElDropdownMenu } from 'element-plus'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import ICheck from '~icons/lucide/check'
import ISettings from '~icons/lucide/settings'

type Entity = 'bottle' | 'prescription' | 'pill' | 'capsule'

const props = defineProps<{
    entity: Entity
    bottleId?: string
    rxId?: string
    pillId?: string
    capsuleId?: string
    /** Required for entity="prescription": only show "reopen" when closed. */
    rxClosed?: boolean
}>()

interface Action {
    key: string
    /** When present, clicking copies this prompt; otherwise the item is disabled. */
    prompt?: string
    /** Adds a "(coming soon)" suffix to the label. */
    comingSoon?: boolean
}

const { t } = useI18n()
const { copy } = useClipboard({ legacy: true })

const copiedKey = ref<string | null>(null)
let copyTimer: ReturnType<typeof setTimeout> | null = null

const promptVars = computed(() => ({
    bottle_id: props.bottleId ? shortId(props.bottleId) : '',
    rx_id: props.rxId ? shortId(props.rxId) : '',
    pill_id: props.pillId ? shortId(props.pillId) : '',
    capsule_id: props.capsuleId ? shortId(props.capsuleId) : '',
}))

function makePrompt(actionKey: string): string {
    return t(`prompts.${props.entity}.${actionKey}.text`, promptVars.value)
}

function labelFor(actionKey: string): string {
    return t(`prompts.${props.entity}.${actionKey}.label`)
}

const actions = computed<Action[]>(() => {
    switch (props.entity) {
        case 'bottle':
            return [
                { key: 'explain', prompt: makePrompt('explain') },
                { key: 'next_session', comingSoon: true },
            ]
        case 'prescription': {
            const items: Action[] = [
                { key: 'explain', prompt: makePrompt('explain') },
                { key: 'summary', prompt: makePrompt('summary') },
            ]
            if (props.rxClosed) {
                items.push({ key: 'reopen', prompt: makePrompt('reopen') })
            }
            return items
        }
        case 'pill':
            return [
                { key: 'explain', prompt: makePrompt('explain') },
                { key: 'audit', prompt: makePrompt('audit') },
                { key: 'improve', prompt: makePrompt('improve') },
            ]
        case 'capsule':
            return [
                { key: 'explain', prompt: makePrompt('explain') },
                { key: 'apply', prompt: makePrompt('apply') },
            ]
    }
})

function onSelect(action: Action) {
    if (!action.prompt) return
    copy(action.prompt)
    copiedKey.value = action.key
    if (copyTimer) clearTimeout(copyTimer)
    copyTimer = setTimeout(() => {
        copiedKey.value = null
        copyTimer = null
    }, 1500)
}
</script>

<template>
    <el-dropdown trigger="click" placement="bottom-end" :hide-on-click="false">
        <button
            type="button"
            class="flex items-center justify-center w-7 h-7 rounded-md border border-(--border) text-zinc-400 hover:text-(--text-h)"
            :title="t('common.options')"
        >
            <ISettings class="size-3.5" />
        </button>
        <template #dropdown>
            <el-dropdown-menu>
                <div class="flex items-center gap-1.5 px-3 py-1.5 font-medium uppercase tracking-wide text-zinc-500 border-b border-(--border) select-none">
                    <span>{{ t('common.prompts_header') }}</span>
                </div>
                <el-dropdown-item
                    v-for="action in actions"
                    :key="action.key"
                    :disabled="!action.prompt"
                    @click="onSelect(action)"
                >
                    <div class="flex items-center gap-2 min-w-0 w-full">
                        <span class="truncate flex-1">
                            {{ labelFor(action.key) }}
                            <span v-if="action.comingSoon" class="ml-1 text-xs text-zinc-500">
                                {{ t('common.coming_soon') }}
                            </span>
                        </span>
                        <ICheck
                            v-if="copiedKey === action.key"
                            class="w-3.5 h-3.5 text-green-500 shrink-0"
                        />
                    </div>
                </el-dropdown-item>
            </el-dropdown-menu>
        </template>
    </el-dropdown>
</template>
