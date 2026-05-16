import { api } from '@/core/infrastructure/managers/httpClient'
import type { Paginated, PaginationParams, Pill, Prescription } from '@/core/domain/types'

export const prescriptionsApi = {
    get: (bottleId: string, rxId: string) =>
        api.get<Prescription>(`/bottles/${bottleId}/prescriptions/${rxId}`),
    open: (bottleId: string, body: { title: string }) =>
        api.post<Prescription>(`/bottles/${bottleId}/prescriptions`, body),
    close: (bottleId: string, rxId: string) =>
        api.patch<Prescription>(`/bottles/${bottleId}/prescriptions/${rxId}`),
    reopen: (bottleId: string, rxId: string) =>
        api.post<Prescription>(`/bottles/${bottleId}/prescriptions/${rxId}/reopen`, {}),
    delete: (bottleId: string, rxId: string) =>
        api.delete<{ discarded: boolean }>(`/bottles/${bottleId}/prescriptions/${rxId}`),
    purge: (bottleId: string, rxId: string) =>
        api.delete<{ purged: boolean }>(`/bottles/${bottleId}/prescriptions/${rxId}/purge`),
    pills: (bottleId: string, rxId: string, pagination: PaginationParams) => {
        const qs = new URLSearchParams({
            page: String(pagination.page),
            page_size: String(pagination.page_size),
        })
        return api.get<Paginated<Pill>>(`/bottles/${bottleId}/prescriptions/${rxId}/pills?${qs}`)
    },
}
