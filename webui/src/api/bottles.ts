import { api } from './client'
import type { Bottle, Prescription } from './types'

export const bottlesApi = {
    list: () => api.get<Bottle[]>('/bottles'),
    get: (id: number) => api.get<Bottle>(`/bottles/${id}`),
    create: (body: { name: string; display_name: string; directory: string; scope: 'local' | 'global' }) =>
        api.post<Bottle>('/bottles', body),
    prescriptions: (id: number, limit = 50) =>
        api.get<Prescription[]>(`/bottles/${id}/prescriptions?limit=${limit}`),
}
