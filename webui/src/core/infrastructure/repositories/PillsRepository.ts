import { api } from '@/core/infrastructure/managers/httpClient'
import type { Pill, PillSearchResult, Compound } from '@/core/domain/types'

export const pillsApi = {
    get: (pillId: string, bottleId: string, rxId: string) =>
        api.get<Pill>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}`),
    update: (pillId: string, body: { title?: string; content?: string; compound?: string }, bottleId: string, rxId: string) =>
        api.patch<Pill>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}`, body),
    delete: (pillId: string, bottleId: string, rxId: string) =>
        api.delete<Pill>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}`),
    purge: (pillId: string, bottleId: string, rxId: string) =>
        api.delete<{ purged: boolean }>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}/purge`),
    search: (params: { query: string; bottle_id?: string; compound?: string; limit?: number; fuzzy?: boolean }) => {
        const q = new URLSearchParams({ query: params.query })
        if (params.bottle_id) q.set('bottle_id', params.bottle_id)
        if (params.compound) q.set('compound', params.compound)
        if (params.limit) q.set('limit', String(params.limit))
        if (params.fuzzy !== undefined) q.set('fuzzy', String(params.fuzzy))
        return api.get<PillSearchResult[]>(`/pills/search?${q}`)
    },
    getCompounds: (bottleId?: string, limit?: number) => {
        const q = new URLSearchParams()
        if (bottleId) q.set('bottle_id', bottleId)
        if (limit) q.set('limit', String(limit))
        const qs = q.toString()
        return api.get<Compound[]>(`/pills/compounds${qs ? '?' + qs : ''}`)
    },
}
