// ──── NL Dates V1 — quilt-feature-natural-dates-v1 ────────────
//
// Convert natural-language date tokens ("today", "tomorrow",
// "yesterday") to ISO YYYY-MM-DD strings for use in property values.
//
// V1 scope: pure frontend transform. The user types `deadline:: today`
// in the inline editor; before persistence we rewrite it to
// `deadline:: 2026-06-05` so the backend always sees a real ISO date.
//
// V2 (not this file) would extend the DSL parser with `@today`,
// `@tomorrow` literals so the *raw* content preserves the intent and
// the resolver runs at query time. V1 keeps the resolver at the UI
// boundary — simpler, no schema migration, no DSL changes.
//
// Why a dedicated module (not inline in BlockRow.saveToApi)?
//   - It's pure: easy to unit-test deterministically with a `refDate`.
//   - It's the kind of helper that grows over time (next week, +3d…).
//   - Keeps the inline content layer (InlineContent.tsx) free of date
//     arithmetic, and the property rendering free of format quirks.
//
// All public functions are pure — no globals, no Date.now() at
// import-time, no timezone juggling beyond "local" components.
// That last point is deliberate: "today" means *the user's local
// today*, not UTC today, so someone in PST typing at 11pm shouldn't
// see tomorrow's date appear in their property.

/** Canonical property keys whose values are dates. Lowercased. */
const DATE_PROPERTY_KEYS = ['deadline', 'scheduled', 'date'] as const

/**
 * Resolve a single natural-language date token to a YYYY-MM-DD string.
 *
 * Accepts (case-insensitive, whitespace-trimmed):
 *   - `"today"`     → the reference date
 *   - `"tomorrow"`  → reference date + 1 day
 *   - `"yesterday"` → reference date − 1 day
 *
 * Returns `null` for any other input — including real ISO dates
 * (`"2026-01-15"`), empty strings, and compound tokens
 * (`"today!"`, `"today afternoon"`). The strict equality is what lets
 * us safely scan inline content without rewriting free-text mentions.
 *
 * @param input  The token to resolve. Typically a single word.
 * @param refDate  The "now" reference. Defaults to the current
 *                 `new Date()`. Tests pass a fixed date for
 *                 determinism — never read the wall clock in tests.
 */
export function resolveNaturalDate(
  input: string,
  refDate?: Date,
): string | null {
  if (typeof input !== 'string') return null
  const token = input.trim().toLowerCase()
  if (!token) return null

  if (token !== 'today' && token !== 'tomorrow' && token !== 'yesterday') {
    return null
  }

  const base = refDate ?? new Date()
  // `setDate` handles month/year overflow naturally
  // (setDate(0) → last day of previous month,
  //  setDate(32) on Dec 31 → Jan 1 of next year).
  const offsetDays = token === 'tomorrow' ? 1 : token === 'yesterday' ? -1 : 0
  const date = new Date(base)
  date.setDate(date.getDate() + offsetDays)

  return formatYmd(date)
}

/** Format a Date as `YYYY-MM-DD` using *local* components.
 *  Local (not UTC) is correct here because the user thinks in local
 *  time and "today" means local today. */
function formatYmd(d: Date): string {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

/**
 * Type guard: is this property key one that holds a date value?
 *
 * Recognises `deadline`, `scheduled`, and `date` (case-insensitive).
 * These are the keys the slash command menu already inserts
 * (see `makePropertyHandler` in slashRegistry.tsx).
 */
export function isDatePropertyKey(key: string): boolean {
  if (typeof key !== 'string') return false
  return (DATE_PROPERTY_KEYS as readonly string[]).includes(
    key.trim().toLowerCase(),
  )
}

/**
 * Scan a block's inline content and resolve any natural-date tokens
 * that appear as values of date properties (`deadline::`, `scheduled::`,
 * `date::`).
 *
 * Behaviour:
 *   - Only rewrites values that are *exactly* a natural-date token
 *     (post-trim). Free-text mentions like `"see you tomorrow!"` are
 *     left untouched.
 *   - Only rewrites values inside a known date property. `status::`
 *     stays a string.
 *   - Preserves surrounding whitespace, leading indentation, and
 *     other lines in multi-line content.
 *
 * This is the function BlockRow.saveToApi calls before `api.updateBlock`.
 *
 * @param content  The raw block content (the same string the user
 *                 typed in the contentEditable).
 * @param refDate  Optional "now" for determinism. Defaults to
 *                 `new Date()`.
 */
export function resolveNaturalDatesInContent(
  content: string,
  refDate?: Date,
): string {
  if (!content) return content

  return content
    .split('\n')
    .map(line => resolveNaturalDateInLine(line, refDate))
    .join('\n')
}

/** Per-line transform. Splits out for readability and easy testing. */
function resolveNaturalDateInLine(line: string, refDate?: Date): string {
  // Capture: 1=leading-ws, 2=key, 3=sep-ws, 4=value, 5=trailing-ws.
  // The value is `[^\s][^\n]*?` (starts with non-ws, non-greedy until
  // the trailing-whitespace+end anchor) so we don't eat the
  // separator's whitespace.
  const m = line.match(
    /^(\s*)(deadline|scheduled|date)::(\s*)([^\s][^\n]*?)(\s*)$/i,
  )
  if (!m) return line

  const [, leading, key, sepWs, value, trailingWs] = m
  const resolved = resolveNaturalDate(value, refDate)
  if (resolved === null) return line

  return `${leading}${key}::${sepWs}${resolved}${trailingWs}`
}
