import { api } from '@/core/infrastructure/managers/httpClient'
import type { Pill, PillSearchResult } from '@/core/domain/types'

export const pillsApi = {
    get: (pillId: number, bottleId: string, rxId: string) =>
        api.get<Pill>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}`),
    update: (pillId: number, body: { title?: string; content?: string; compound?: string }, bottleId: string, rxId: string) =>
        api.patch<Pill>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}`, body),
    delete: (pillId: number, bottleId: string, rxId: string) =>
        api.delete<Pill>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}`),
    purge: (pillId: number, bottleId: string, rxId: string) =>
        api.delete<{ purged: boolean }>(`/bottles/${bottleId}/prescriptions/${rxId}/pills/${pillId}/purge`),
    search: (params: { query: string; bottle_id?: string; compound?: string; limit?: number }) => {
        const q = new URLSearchParams({ query: params.query })
        if (params.bottle_id) q.set('bottle_id', params.bottle_id)
        if (params.compound) q.set('compound', params.compound)
        if (params.limit) q.set('limit', String(params.limit))
        return api.get<PillSearchResult[]>(`/pills/search?${q}`)
    },
}
