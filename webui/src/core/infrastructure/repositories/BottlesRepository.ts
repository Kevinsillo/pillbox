import { api } from '@/core/infrastructure/managers/httpClient'
import type { Bottle, BottleStats, Paginated, PaginationParams, Prescription } from '@/core/domain/types'

export const bottlesApi = {
    list: (pagination: PaginationParams) => {
        const qs = new URLSearchParams({
            page: String(pagination.page),
            page_size: String(pagination.page_size),
        })
        return api.get<Paginated<Bottle>>(`/bottles?${qs}`)
    },
    get: (id: string) => api.get<Bottle>(`/bottles/${id}`),
    create: (body: { name: string; display_name: string; directory: string; scope: 'local' | 'global' }) =>
        api.post<Bottle>('/bottles', body),
    prescriptions: (id: string, pagination: PaginationParams) => {
        const qs = new URLSearchParams({
            page: String(pagination.page),
            page_size: String(pagination.page_size),
        })
        return api.get<Paginated<Prescription>>(`/bottles/${id}/prescriptions?${qs}`)
    },
    updateRegistration: (regId: number, db_path: string) => api.patch<null>(`/registered_bottles/${regId}`, { db_path }),
    deleteBottle: (id: string) => api.delete<null>(`/bottles/${id}`),
    deleteRegistration: (regId: number) => api.delete<null>(`/registered_bottles/${regId}`),
    stats: (id: string, days = 30) => api.get<BottleStats>(`/bottles/${id}/stats?days=${days}`),
}
