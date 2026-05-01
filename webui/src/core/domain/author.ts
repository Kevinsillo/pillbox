/**
 * Format author name and email into a single display string.
 *
 * Composite format:
 *   - both present → "Name <email>"
 *   - only name    → "Name"
 *   - only email   → "email"
 *   - both null    → null (caller should omit the element entirely)
 */
export function formatAuthor(name: string | null, email: string | null): string | null {
    if (name && email) return `${name} <${email}>`
    if (name) return name
    if (email) return email
    return null
}
