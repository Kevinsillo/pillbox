import { api } from '@/core/infrastructure/managers/httpClient'
import type { Capsule, CapsuleSummary, CapsuleSearchResult } from '@/core/domain/types'

export const capsulesApi = {
    list: (params?: { compound?: string; limit?: number }) => {
        const q = new URLSearchParams()
        if (params?.compound) q.set('compound', params.compound)
        if (params?.limit) q.set('limit', String(params.limit))
        const qs = q.toString()
        return api.get<CapsuleSummary[]>(`/capsules${qs ? '?' + qs : ''}`)
    },
    get: (id: number) => api.get<Capsule>(`/capsules/${id}`),
    create: (body: { title: string; content: string; compound: string }) =>
        api.post<Capsule>('/capsules', body),
    update: (id: number, body: { title?: string; content?: string; compound?: string }) =>
        api.patch<Capsule>(`/capsules/${id}`, body),
    delete: (id: number) => api.delete<Capsule>(`/capsules/${id}`),
    purge: (id: number) => api.delete<{ purged: boolean }>(`/capsules/${id}/purge`),
    search: (params: { query: string; compound?: string; limit?: number }) => {
        const qs = new URLSearchParams({ query: params.query })
        if (params.compound) qs.set('compound', params.compound)
        if (params.limit) qs.set('limit', String(params.limit))
        return api.get<CapsuleSearchResult[]>(`/capsules/search?${qs}`)
    },
}
