import { reactive } from "vue"

interface DialogState {
    visible: boolean
    variant: "confirm" | "prompt" | "alert" | "dual"
    title: string
    message: string
    confirmText: string
    cancelText: string
    danger: boolean
    inputValue: string
    inputPlaceholder: string
    inputValidator?: (v: string) => boolean | string
    waitSeconds: number
    softText: string
    hardText: string
    hardWaitSeconds: number
}

export const confirmState = reactive<DialogState>({
    visible: false,
    variant: "confirm",
    title: "",
    message: "",
    confirmText: "Confirmar",
    cancelText: "Cancelar",
    danger: false,
    inputValue: "",
    inputPlaceholder: "",
    waitSeconds: 0,
    softText: "Archivar",
    hardText: "Eliminar definitivamente",
    hardWaitSeconds: 5,
})

let _resolve: ((value?: string) => void) | undefined
let _reject: (() => void) | undefined
let _resolveDual: ((mode: "soft" | "hard") => void) | undefined
let _rejectDual: (() => void) | undefined

export function resolveDialog(value?: string) {
    _resolve?.(value)
    _resolve = undefined
    _reject = undefined
    confirmState.visible = false
}

export function rejectDialog() {
    _reject?.()
    _resolve = undefined
    _reject = undefined
    confirmState.visible = false
}

export function resolveDualDialog(mode: "soft" | "hard") {
    _resolveDual?.(mode)
    _resolveDual = undefined
    _rejectDual = undefined
    confirmState.visible = false
}

export function rejectDualDialog() {
    _rejectDual?.()
    _resolveDual = undefined
    _rejectDual = undefined
    confirmState.visible = false
}

export function useConfirm() {
    function confirm(
        message: string,
        title?: string | null,
        opts?: { confirmText?: string; cancelText?: string; waitSeconds?: number }
    ): Promise<void> {
        return new Promise((resolve, reject) => {
            _resolve = resolve as (value?: string) => void
            _reject = reject
            Object.assign(confirmState, {
                visible: true,
                variant: "confirm",
                title: title ?? "",
                message,
                confirmText: opts?.confirmText ?? "Confirmar",
                cancelText: opts?.cancelText ?? "Cancelar",
                danger: true,
                waitSeconds: opts?.waitSeconds ?? 0,
            })
        })
    }

    function prompt(
        message: string,
        title?: string | null,
        opts?: {
            inputValue?: string
            inputPlaceholder?: string
            inputValidator?: (v: string) => boolean | string
            confirmText?: string
            cancelText?: string
        }
    ): Promise<{ value: string }> {
        return new Promise((resolve, reject) => {
            _resolve = (value?: string) => resolve({ value: value ?? "" })
            _reject = reject
            Object.assign(confirmState, {
                visible: true,
                variant: "prompt",
                title: title ?? "",
                message,
                confirmText: opts?.confirmText ?? "Confirmar",
                cancelText: opts?.cancelText ?? "Cancelar",
                danger: false,
                inputValue: opts?.inputValue ?? "",
                inputPlaceholder: opts?.inputPlaceholder ?? "",
                inputValidator: opts?.inputValidator,
            })
        })
    }

    function alert(message: string): Promise<void> {
        return new Promise((resolve) => {
            _resolve = resolve as (value?: string) => void
            _reject = undefined
            Object.assign(confirmState, {
                visible: true,
                variant: "alert",
                title: "",
                message,
                confirmText: "OK",
                cancelText: "",
                danger: false,
            })
        })
    }

    function confirmDual(
        message: string,
        title?: string | null,
        opts?: { softText?: string; hardText?: string; hardWaitSeconds?: number; cancelText?: string }
    ): Promise<"soft" | "hard"> {
        return new Promise((resolve, reject) => {
            _resolveDual = resolve
            _rejectDual = reject
            Object.assign(confirmState, {
                visible: true,
                variant: "dual",
                title: title ?? "",
                message,
                cancelText: opts?.cancelText ?? "Cancelar",
                softText: opts?.softText ?? "Archivar",
                hardText: opts?.hardText ?? "Eliminar definitivamente",
                hardWaitSeconds: opts?.hardWaitSeconds ?? 5,
            })
        })
    }

    return { confirm, prompt, alert, confirmDual }
}
