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

/**
 * Format a UTC date-time string ("YYYY-MM-DD HH:MM:SS") as a local date "dd-mm-yyyy".
 *
 * The input is interpreted as UTC (a 'T' separator and a missing time part are
 * tolerated; missing time means 00:00:00) and converted to the user's local
 * timezone before extracting the day. Returns '' for null/undefined/unparseable input.
 */
export function formatDateDMY(value: string | null | undefined): string {
    if (value == null) return ""
    const m = /^(\d{4})-(\d{2})-(\d{2})(?:[ T](\d{2}):(\d{2}):(\d{2}))?/.exec(value.trim())
    if (!m) return ""
    const [, y, mo, d, h = "0", mi = "0", s = "0"] = m
    const date = new Date(Date.UTC(Number(y), Number(mo) - 1, Number(d), Number(h), Number(mi), Number(s)))
    if (Number.isNaN(date.getTime())) return ""
    const pad = (n: number) => String(n).padStart(2, "0")
    return `${pad(date.getDate())}-${pad(date.getMonth() + 1)}-${date.getFullYear()}`
}
