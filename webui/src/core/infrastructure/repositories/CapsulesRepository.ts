import { api } from '@/core/infrastructure/managers/httpClient'
import type { Capsule, CapsuleSearchResult, Compound, Paginated, PaginationParams } from '@/core/domain/types'

export const capsulesApi = {
    list: (pagination: PaginationParams, compound?: string) => {
        const qs = new URLSearchParams({
            page: String(pagination.page),
            page_size: String(pagination.page_size),
        })
        if (compound) qs.set('compound', compound)
        return api.get<Paginated<Capsule>>(`/capsules?${qs}`)
    },
    get: (id: string) => api.get<Capsule>(`/capsules/${id}`),
    create: (body: { title: string; content: string; compound: string }) =>
        api.post<Capsule>('/capsules', body),
    update: (id: string, body: { title?: string; content?: string; compound?: string }) =>
        api.patch<Capsule>(`/capsules/${id}`, body),
    delete: (id: string) => api.delete<Capsule>(`/capsules/${id}`),
    purge: (id: string) => api.delete<{ purged: boolean }>(`/capsules/${id}/purge`),
    search: (params: { query: string; compound?: string; page: number; page_size: number }) => {
        const qs = new URLSearchParams({
            query: params.query,
            page: String(params.page),
            page_size: String(params.page_size),
        })
        if (params.compound) qs.set('compound', params.compound)
        return api.get<Paginated<CapsuleSearchResult>>(`/capsules/search?${qs}`)
    },
    getCompounds: (limit?: number) => {
        const q = new URLSearchParams()
        if (limit) q.set('limit', String(limit))
        const qs = q.toString()
        return api.get<Compound[]>(`/capsules/compounds${qs ? '?' + qs : ''}`)
    },
}
