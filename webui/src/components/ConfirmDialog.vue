<script setup lang="ts">
import { confirmState, rejectDialog, resolveDialog, resolveDualDialog, rejectDualDialog } from "@/composables/useConfirm"
import { ElInput } from "element-plus"
import { ref, watch } from "vue"

const inputValue = ref("")
const inputError = ref("")
const remaining = ref(0)
const hardRemaining = ref(0)
let _timer: ReturnType<typeof setInterval> | undefined
let _hardTimer: ReturnType<typeof setInterval> | undefined

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
            if (confirmState.variant === "dual") {
                hardRemaining.value = confirmState.hardWaitSeconds
                if (hardRemaining.value > 0) {
                    _hardTimer = setInterval(() => {
                        hardRemaining.value--
                        if (hardRemaining.value <= 0) {
                            clearInterval(_hardTimer)
                            _hardTimer = undefined
                        }
                    }, 1000)
                }
            }
        } else {
            clearInterval(_timer)
            _timer = undefined
            remaining.value = 0
            clearInterval(_hardTimer)
            _hardTimer = undefined
            hardRemaining.value = 0
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
    } else if (confirmState.variant === "dual") {
        rejectDualDialog()
    } else {
        rejectDialog()
    }
}

function handleSoft() {
    resolveDualDialog("soft")
}

function handleHard() {
    resolveDualDialog("hard")
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

                        <!-- Dual variant buttons -->
                        <div v-if="confirmState.variant === 'dual'" class="flex justify-end gap-2">
                            <button
                                class="border border-(--border) px-3 py-2 rounded-lg text-sm text-zinc-400 hover:text-(--text-h) transition-colors"
                                @click="handleCancel"
                            >
                                {{ confirmState.cancelText }}
                            </button>
                            <button
                                class="border border-red-900/40 px-3 py-2 rounded-lg text-sm text-red-400 hover:text-red-300 transition-colors"
                                @click="handleSoft"
                            >
                                {{ confirmState.softText }}
                            </button>
                            <button
                                :disabled="hardRemaining > 0"
                                :class="
                                    hardRemaining > 0
                                        ? 'border border-(--border) text-zinc-600 cursor-not-allowed'
                                        : 'border border-red-900/40 text-red-400 hover:text-red-300'
                                "
                                class="px-3 py-2 rounded-lg text-sm transition-colors"
                                @click="handleHard"
                            >
                                {{ confirmState.hardText }}{{ hardRemaining > 0 ? ` (${hardRemaining})` : "" }}
                            </button>
                        </div>

                        <!-- Default variant buttons -->
                        <div v-else class="flex justify-end gap-2">
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
