import { api } from './client'
import type { Prescription, Pill } from './types'

export const prescriptionsApi = {
    get: (id: string) => api.get<Prescription>(`/prescriptions/${id}`),
    open: (body: { bottle_id: number; title: string }) => api.post<Prescription>('/prescriptions', body),
    close: (id: string) => api.patch<Prescription>(`/prescriptions/${id}`),
    delete: (id: string) => api.delete<{ discarded: boolean }>(`/prescriptions/${id}`),
    pills: (id: string) => api.get<Pill[]>(`/prescriptions/${id}/pills`),
}
