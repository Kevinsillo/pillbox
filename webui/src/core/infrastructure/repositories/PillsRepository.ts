import { api } from '@/core/infrastructure/managers/httpClient'
import type { Compound, Paginated, Pill, PillSearchResult } from '@/core/domain/types'

export const pillsApi = {
    get: (pillId: string, bottleId: string, rxId: string) =>
        api.get<Pill>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}`),
    update: (pillId: string, body: { title?: string; content?: string; compound?: string }, bottleId: string, rxId: string) =>
        api.patch<Pill>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}`, body),
    delete: (pillId: string, bottleId: string, rxId: string) =>
        api.delete<Pill>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}`),
    purge: (pillId: string, bottleId: string, rxId: string) =>
        api.delete<{ purged: boolean }>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}/purge`),
    search: (params: { query: string; bottle_id?: string; compound?: string; page: number; page_size: number }) => {
        const q = new URLSearchParams({
            query: params.query,
            page: String(params.page),
            page_size: String(params.page_size),
        })
        if (params.bottle_id) q.set('bottle_id', params.bottle_id)
        if (params.compound) q.set('compound', params.compound)
        return api.get<Paginated<PillSearchResult>>(`/pills/search?${q}`)
    },
    getCompounds: (bottleId?: string, limit?: number) => {
        const q = new URLSearchParams()
        if (bottleId) q.set('bottle_id', bottleId)
        if (limit) q.set('limit', String(limit))
        const qs = q.toString()
        return api.get<Compound[]>(`/pills/compounds${qs ? '?' + qs : ''}`)
    },
}
