import { api } from '@/core/infrastructure/managers/httpClient'

export type AppInfo = {
    version: string
    os: string
    arch: string
    family: string
}

export const metaApi = {
    info: () => api.get<AppInfo>('/info'),
}
