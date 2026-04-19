import { api } from '@/core/infrastructure/managers/httpClient'
import type { Context } from '@/core/domain/types'

export const contextApi = {
    get: (bottle_id: number, params?: { prescription_limit?: number; pill_limit?: number }) => {
        const q = new URLSearchParams({ bottle_id: String(bottle_id) })
        if (params?.prescription_limit) q.set('prescription_limit', String(params.prescription_limit))
        if (params?.pill_limit) q.set('pill_limit', String(params.pill_limit))
        return api.get<Context>(`/context?${q}`)
    },
}
