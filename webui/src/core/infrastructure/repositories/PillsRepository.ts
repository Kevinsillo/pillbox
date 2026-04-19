import { api } from '@/core/infrastructure/managers/httpClient'
import type { Pill, PillSearchResult, PillCompound } from '@/core/domain/types'

export const pillsApi = {
    get: (id: number) => api.get<Pill>(`/pills/${id}`),
    create: (body: {
        title: string
        content: string
        compound: PillCompound
        prescription_id: string
        dispenser?: string
        author_name?: string
        author_email?: string
    }) => api.post<Pill>('/pills', body),
    update: (id: number, body: { title?: string; content?: string; compound?: PillCompound }) =>
        api.patch<Pill>(`/pills/${id}`, body),
    delete: (id: number) => api.delete<Pill>(`/pills/${id}`),
    search: (params: { query: string; bottle_id?: number; compound?: PillCompound; limit?: number }) => {
        const q = new URLSearchParams({ query: params.query })
        if (params.bottle_id) q.set('bottle_id', String(params.bottle_id))
        if (params.compound) q.set('compound', params.compound)
        if (params.limit) q.set('limit', String(params.limit))
        return api.get<PillSearchResult[]>(`/pills/search?${q}`)
    },
}
