<script setup lang="ts">
import { confirmState, rejectDialog, resolveDialog } from "@/composables/useConfirm"
import { ElInput } from "element-plus"
import { ref, watch } from "vue"

const inputValue = ref("")
const inputError = ref("")
const remaining = ref(0)
let _timer: ReturnType<typeof setInterval> | undefined

watch(
    () => confirmState.visible,
    (v) => {
        if (v) {
            inputValue.value = confirmState.inputValue
            inputError.value = ""
            remaining.value = confirmState.waitSeconds
            if (remaining.value > 0) {
                _timer = setInterval(() => {
                    remaining.value--
                    if (remaining.value <= 0) {
                        clearInterval(_timer)
                        _timer = undefined
                    }
                }, 1000)
            }
        } else {
            clearInterval(_timer)
            _timer = undefined
            remaining.value = 0
        }
    }
)

function handleConfirm() {
    if (confirmState.variant === "prompt") {
        const validator = confirmState.inputValidator
        if (validator) {
            const result = validator(inputValue.value)
            if (result !== true) {
                inputError.value = typeof result === "string" ? result : "Valor inválido"
                return
            }
        }
        resolveDialog(inputValue.value)
    } else {
        resolveDialog()
    }
}

function handleCancel() {
    if (confirmState.variant === "alert") {
        resolveDialog()
    } else {
        rejectDialog()
    }
}
</script>

<template>
    <Teleport to="body">
        <Transition
            enter-active-class="transition-opacity duration-150"
            enter-from-class="opacity-0"
            leave-active-class="transition-opacity duration-100"
            leave-to-class="opacity-0"
        >
            <div
                v-if="confirmState.visible"
                class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
                @click.self="handleCancel"
                @keydown.esc="handleCancel"
            >
                <div class="bg-(--bg-surface) border border-(--border) rounded-xl w-full max-w-md shadow-2xl">
                    <div v-if="confirmState.title" class="px-5 py-4 border-b border-(--border)">
                        <h2 class="text-(--text-h) font-semibold">{{ confirmState.title }}</h2>
                    </div>
                    <div class="p-5 space-y-4">
                        <p class="text-sm text-(--text)" v-html="confirmState.message"></p>

                        <div v-if="confirmState.variant === 'prompt'" class="space-y-1">
                            <el-input
                                v-model="inputValue"
                                :placeholder="confirmState.inputPlaceholder"
                                autofocus
                                @keyup.enter="handleConfirm"
                            />
                            <p v-if="inputError" class="text-xs text-red-400">{{ inputError }}</p>
                        </div>

                        <div class="flex justify-end gap-2">
                            <button
                                v-if="confirmState.variant !== 'alert'"
                                class="border border-(--border) px-3 py-2 rounded-lg text-sm text-zinc-400 hover:text-(--text-h) transition-colors"
                                @click="handleCancel"
                            >
                                {{ confirmState.cancelText }}
                            </button>
                            <button
                                :disabled="remaining > 0"
                                :class="
                                    remaining > 0
                                        ? 'border border-(--border) text-zinc-600 cursor-not-allowed'
                                        : confirmState.danger
                                          ? 'border border-red-900/40 text-red-400 hover:text-red-300'
                                          : 'bg-(--accent-bg) hover:bg-zinc-600 text-(--text-h)'
                                "
                                class="px-3 py-2 rounded-lg text-sm transition-colors"
                                @click="handleConfirm"
                            >
                                {{ confirmState.confirmText }}{{ remaining > 0 ? ` (${remaining})` : "" }}
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </Transition>
    </Teleport>
</template>
