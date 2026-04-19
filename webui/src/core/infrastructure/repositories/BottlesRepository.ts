import { api } from '@/core/infrastructure/managers/httpClient'
import type { Bottle, Prescription } from '@/core/domain/types'

export const bottlesApi = {
    list: () => api.get<Bottle[]>('/bottles'),
    get: (id: number) => api.get<Bottle>(`/bottles/${id}`),
    create: (body: { name: string; display_name: string; directory: string; scope: 'local' | 'global' }) =>
        api.post<Bottle>('/bottles', body),
    prescriptions: (id: number, limit = 50) =>
        api.get<Prescription[]>(`/bottles/${id}/prescriptions?limit=${limit}`),
}
