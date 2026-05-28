/**
 * Format an ISO date-time string for display using the user's locale.
 *
 * Behavior-preserving wrapper around `new Date(iso).toLocaleString()`.
 * Returns '' for null/undefined so call-sites stay safe.
 */
export function formatDateTime(iso: string | null | undefined): string {
    if (iso == null) return ""
    return new Date(iso).toLocaleString()
}

/**
 * Format an ISO date-time string as a date-only display string using the user's locale.
 *
 * Behavior-preserving wrapper around `new Date(iso).toLocaleDateString()`.
 * Returns '' for null/undefined so call-sites stay safe.
 */
export function formatDate(iso: string | null | undefined): string {
    if (iso == null) return ""
    return new Date(iso).toLocaleDateString()
}
