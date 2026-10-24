// Display formatting. The store keeps SI units (metres, minutes, UTC);
// conversion to what's shown happens here, in one place.

const locale = undefined // the system locale

export const fmtInt = (n: number | null | undefined) =>
  n == null ? '–' : Math.round(n).toLocaleString(locale)

export const fmtKm = (m: number | null | undefined) => (m == null ? '–' : (m / 1000).toFixed(2))

/** 437 -> "7h 17m" */
export function fmtMinutes(min: number | null | undefined): string {
  if (min == null) return '–'
  const h = Math.floor(min / 60)
  const m = Math.round(min % 60)
  return h > 0 ? `${h}h ${String(m).padStart(2, '0')}m` : `${m}m`
}

/** "2026-10-24T19:57:00Z" -> "08:57" (local) */
export const fmtTime = (utc: string) =>
  new Date(utc).toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' })

/** "2026-10-24T19:57:00Z" -> "Fri 24 Oct" (local) */
export const fmtDay = (utc: string) =>
  new Date(utc).toLocaleDateString(locale, { weekday: 'short', day: 'numeric', month: 'short' })

/** "2026-10-24" (civil) -> "24 Oct" */
export function fmtCivil(date: string): string {
  const [y, m, d] = date.split('-').map(Number)
  return new Date(y, m - 1, d).toLocaleDateString(locale, { day: 'numeric', month: 'short' })
}

/** "2026-10-24" (civil) -> "Saturday 24 October" */
export function fmtLongDate(date: string): string {
  const [y, m, d] = date.split('-').map(Number)
  return new Date(y, m - 1, d).toLocaleDateString(locale, { weekday: 'long', day: 'numeric', month: 'long' })
}

/** "12 min ago", "3 h ago", "2 days ago" */
export function fmtAgo(utc: string | null | undefined, now = Date.now()): string {
  if (!utc) return 'never'
  const mins = Math.floor((now - new Date(utc).getTime()) / 60000)
  if (mins < 1) return 'just now'
  if (mins < 60) return `${mins} min ago`
  if (mins < 1440) return `${Math.floor(mins / 60)} h ago`
  return `${Math.floor(mins / 1440)} days ago`
}

/** Every civil date from start to end inclusive, as YYYY-MM-DD. */
export function civilDays(start: string, end: string): string[] {
  const [y, m, d] = start.split('-').map(Number)
  const day = new Date(y, m - 1, d)
  const out: string[] = []
  for (;;) {
    const s = `${day.getFullYear()}-${String(day.getMonth() + 1).padStart(2, '0')}-${String(day.getDate()).padStart(2, '0')}`
    out.push(s)
    if (s >= end) return out
    day.setDate(day.getDate() + 1)
  }
}

/** "STRENGTH_TRAINING" -> "Strength training" */
export function fmtEnum(s: string | null | undefined): string {
  if (!s) return ''
  const lower = s.toLowerCase().replace(/_/g, ' ')
  return lower.charAt(0).toUpperCase() + lower.slice(1)
}
