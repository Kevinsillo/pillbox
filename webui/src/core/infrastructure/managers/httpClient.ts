import { i18n } from '@/core/infrastructure/i18n'

const BASE = (import.meta.env.VITE_API_BASE as string | undefined) ?? '/api'

export class ApiError extends Error {
    code: string
    data: unknown
    constructor(code: string, message: string, data?: unknown) {
        super(message)
        this.code = code
        this.data = data
        this.name = 'ApiError'
    }
}

function translateApiError(code: string): string {
    const key = `api_errors.${code}`
    const msg = i18n.global.t(key)
    return msg !== key ? msg : i18n.global.t('api_errors.unknown')
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const res = await fetch(`${BASE}${path}`, {
        method,
        headers: body ? { 'Content-Type': 'application/json' } : {},
        body: body ? JSON.stringify(body) : undefined,
    })
    const json = await res.json()
    if (!json.ok) throw new ApiError(json.error ?? 'unknown', translateApiError(json.error ?? 'unknown'), json.data)
    return json.data as T
}

export const api = {
    get: <T>(path: string) => request<T>('GET', path),
    post: <T>(path: string, body: unknown) => request<T>('POST', path, body),
    patch: <T>(path: string, body?: unknown) => request<T>('PATCH', path, body),
    delete: <T>(path: string) => request<T>('DELETE', path),
}
