// ──── Journal date formatter ───────────────────────────────────────
//
// strftime-like pattern replacement. We intentionally keep this
// dependency-free (no date-fns / dayjs) because the rest of the
// journal pipeline runs in the WASM engine, and shipping a 70 kB
// library to format a header would dwarf the savings.
//
// Supported placeholders:
//   %Y — 4-digit year           (2026)
//   %m — 2-digit month          (06)
//   %d — 2-digit day            (15)
//   %b — abbreviated month name (Jun)
//   %B — full month name        (June)
//   %A — full weekday name      (Monday)
//
// Anything else is passed through unchanged. Unrecognised placeholders
// are left as-is so a user mistake shows up in the UI rather than
// silently disappearing.

const MONTHS_SHORT = [
  'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
  'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec',
] as const

const MONTHS_LONG = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December',
] as const

const WEEKDAYS_LONG = [
  'Sunday', 'Monday', 'Tuesday', 'Wednesday',
  'Thursday', 'Friday', 'Saturday',
] as const

export function formatJournalDate(date: Date, format: string): string {
  const year = date.getFullYear().toString()
  const month = (date.getMonth() + 1).toString().padStart(2, '0')
  const day = date.getDate().toString().padStart(2, '0')
  const monthShort = MONTHS_SHORT[date.getMonth()]
  const monthLong = MONTHS_LONG[date.getMonth()]
  const weekdayLong = WEEKDAYS_LONG[date.getDay()]

  return format
    .replace(/%Y/g, year)
    .replace(/%m/g, month)
    .replace(/%d/g, day)
    .replace(/%b/g, monthShort)
    .replace(/%B/g, monthLong)
    .replace(/%A/g, weekdayLong)
}
