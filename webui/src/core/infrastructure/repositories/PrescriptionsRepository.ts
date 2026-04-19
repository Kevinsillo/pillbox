import { api } from '@/core/infrastructure/managers/httpClient'
import type { Prescription, Pill } from '@/core/domain/types'

export const prescriptionsApi = {
    get: (id: string) => api.get<Prescription>(`/prescriptions/${id}`),
    open: (body: { bottle_id: number; title: string }) => api.post<Prescription>('/prescriptions', body),
    close: (id: string) => api.patch<Prescription>(`/prescriptions/${id}`),
    delete: (id: string) => api.delete<{ discarded: boolean }>(`/prescriptions/${id}`),
    pills: (id: string) => api.get<Pill[]>(`/prescriptions/${id}/pills`),
}
