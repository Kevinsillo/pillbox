export interface Session {
    id: string
    project: string
    directory: string
    started_at: string
    ended_at: string | null
    summary: string | null
}

export interface Observation {
    id: number
    session_id: string
    type: string
    title: string
    project: string
    scope: string
    topic_key: string | null
    revision_count: number
    created_at: string
    updated_at: string
}

export interface ObservationDetail extends Observation {
    content: string
    tool_name: string | null
    duplicate_count: number
    last_seen_at: string | null
}

export interface PaginatedResponse<T> {
    items: T[]
    total: number
    limit: number
    offset: number
}

export interface ApiResponse<T> {
    status: 'success' | 'error'
    message: string
    data: T
}
