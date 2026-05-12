// UUID v7 codifica el timestamp en milisegundos en los primeros 48 bits (12 hex
// chars). Mostrar solo 8 chars (32 bits) causa colisiones para registros creados
// en la misma ventana de ~65 segundos. Con 12 hex chars (sin guiones) cubrimos
// el timestamp completo → colisiones solo si dos registros se crean en el mismo
// milisegundo exacto, prácticamente imposible en uso normal.
//
// Ejemplo: "019e1d52-1a89-7d41-b9c0-1cc0f730b512" → "019e1d521a89"
export function shortId(uuid: string): string {
    return uuid.replace(/-/g, '').slice(0, 12)
}
