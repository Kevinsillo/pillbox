import { api } from '@/core/infrastructure/managers/httpClient'
import type { Prescription, Pill } from '@/core/domain/types'

export const prescriptionsApi = {
    get: (bottleId: string, rxId: string) =>
        api.get<Prescription>(`/bottles/${bottleId}/prescriptions/${rxId}`),
    open: (bottleId: string, body: { title: string }) =>
        api.post<Prescription>(`/bottles/${bottleId}/prescriptions`, body),
    close: (bottleId: string, rxId: string) =>
        api.patch<Prescription>(`/bottles/${bottleId}/prescriptions/${rxId}`),
    delete: (bottleId: string, rxId: string) =>
        api.delete<{ discarded: boolean }>(`/bottles/${bottleId}/prescriptions/${rxId}`),
    purge: (bottleId: string, rxId: string) =>
        api.delete<{ purged: boolean }>(`/bottles/${bottleId}/prescriptions/${rxId}/purge`),
    pills: (bottleId: string, rxId: string) =>
        api.get<Pill[]>(`/bottles/${bottleId}/prescriptions/${rxId}/pills`),
}
