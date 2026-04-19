import { api } from '@/core/infrastructure/managers/httpClient'

export const metaApi = {
    version: () => api.get<{ version: string }>('/version'),
}
