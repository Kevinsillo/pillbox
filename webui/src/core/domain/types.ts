export type PillCompound =
    | 'decision'
    | 'architecture'
    | 'bugfix'
    | 'pattern'
    | 'discovery'
    | 'learning'
    | 'feedback'
    | 'prescription_summary'
    | 'manual'

export type CapsuleCompound =
    | 'convention'
    | 'workflow'
    | 'environment'
    | 'context'
    | 'goal'
    | 'feedback'
    | 'manual'

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
    started_at: string
    ended_at: string | null
    deleted_at: string | null
}

export interface Pill {
    id: number
    sync_id: string
    compound: PillCompound
    title: string
    content: string
    prescription_id: string
    dispenser: string | null
    author_name: string | null
    author_email: string | null
    created_at: string
    updated_at: string
    deleted_at: string | null
}

export interface PillSearchResult {
    id: number
    sync_id: string
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
    id: number
    sync_id: string
    compound: CapsuleCompound
    title: string
    content: string
    created_at: string
    updated_at: string
    deleted_at: string | null
}

export interface CapsuleSummary {
    id: number
    sync_id: string
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
