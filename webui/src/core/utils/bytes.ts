/**
 * Convert a raw byte count into a human-readable string using binary prefixes (base 1024).
 *
 * Rules (per specification):
 * - 0 → "0 B"
 * - 1..1023 → "<n> B" (no decimal)
 * - 1024..1024²-1 → "<x.x> KB" (1 decimal)
 * - 1024²..1024³-1 → "<x.x> MB" (1 decimal)
 * - >= 1024³ → "<x.x> GB" (1 decimal)
 */
export function formatBytes(n: number): string {
    if (!Number.isFinite(n) || n <= 0) return "0 B"
    const KB = 1024
    const MB = KB * 1024
    const GB = MB * 1024
    if (n < KB) return `${Math.trunc(n)} B`
    if (n < MB) return `${(n / KB).toFixed(1)} KB`
    if (n < GB) return `${(n / MB).toFixed(1)} MB`
    return `${(n / GB).toFixed(1)} GB`
}
