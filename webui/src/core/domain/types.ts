export type PillCompound = string
export type CapsuleCompound = string

export interface Compound {
    compound: string
    count: number
}

export interface Bottle {
    id: string
    name: string
    display_name: string
    directory: string
    scope: 'local' | 'global'
    created_at: string
    last_seen_at: string
    linked: boolean
    reg_id?: number
}

export interface Prescription {
    id: string
    bottle_id: string
    title: string
    author_name: string | null
    author_email: string | null
    started_at: string
    ended_at: string | null
    deleted_at: string | null
}

export interface Pill {
    id: string
    compound: PillCompound
    title: string
    content: string
    prescription_id: string
    author_name: string | null
    author_email: string | null
    created_at: string
    updated_at: string
    deleted_at: string | null
}

export interface PillSearchResult {
    id: string
    compound: PillCompound
    title: string
    snippet: string
    created_at: string
    updated_at: string
    rank: number
    prescription_id?: string
    bottle_id?: string
}

export interface Capsule {
    id: string
    compound: CapsuleCompound
    title: string
    content: string
    created_at: string
    updated_at: string
    deleted_at: string | null
}

export interface CapsuleSummary {
    id: string
    compound: CapsuleCompound
    title: string
    created_at: string
    updated_at: string
    deleted_at: string | null
}

export interface CapsuleSearchResult extends CapsuleSummary {
    snippet: string
}

export interface Context {
    context: Pill[]
    prescription_count: number
    pill_count: number
}

export interface DayCount {
    date: string
    count: number
}

export interface BottleStats {
    open_rx_pill_count: number
    closed_rx_count: number
    pills_per_day: DayCount[]
}

export interface ApiOk<T> {
    ok: true
    data: T
}

export interface ApiError {
    ok: false
    error: string
    message: string
    data?: unknown
}

export type ApiResponse<T> = ApiOk<T> | ApiError

export interface PaginationParams {
    page: number
    page_size: number
}

export interface Paginated<T> {
    items: T[]
    total: number
    page: number
    page_size: number
}
