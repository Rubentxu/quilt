// ──── Unlinked Ref Queue (Q029) ────────────────────────────────────
//
// A potential reference is a block that mentions a page name in plain
// prose (e.g. "...see Demo Page for details") WITHOUT wrapping it in
// the `[[...]]` wiki-link syntax. The queue surfaces these candidates
// so the user can review them and either promote them to real links
// (Link action → PUTs `[[Demo Page]]` into the block) or dismiss
// them (Dismiss action → removes them from the queue).
//
// All state is frontend-only: the queue is persisted in localStorage
// under STORAGE_KEY and reconciled with the backend on every page
// open. This file owns the **pure** parts of the feature: detection
// and mutation. The React hook that orchestrates the API calls lives
// next door at `useUnlinkedRefQueue.ts`.

/**
 * One unlinked reference: a block that mentions a page name but does
 * not link to it. `position` is the char offset of the mention inside
 * the block's `content` string; we keep it so the Link action can
 * verify the mention is still there before mutating (defensive — the
 * user might have edited the block between detection and action).
 */
export interface UnlinkedCandidate {
  blockId: string
  pageName: string
  /** The exact substring in `content` that matched. */
  mentionText: string
  /** Char offset of `mentionText` inside the block's content. */
  position: number
  /** When the candidate was first detected (epoch ms). */
  createdAt: number
}

/** localStorage key — single flat array, not a per-page map. */
export const STORAGE_KEY = 'unlinked-ref-queue'

// ──── Detection ──────────────────────────────────────────────────

/**
 * Escape a string for safe inclusion in a `RegExp` constructor.
 * We don't want `C++` or `page.name` to be interpreted as regex
 * metacharacters — the match must be a literal substring.
 */
function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/**
 * Find every plain-text mention of `pageName` in `content`, skipping
 * any occurrence that is already part of a `[[...]]` wiki-link or
 * that sits inside a longer word (e.g. we don't want "Demo Page" to
 * match inside "MyDemo Pagework").
 *
 * Algorithm: scan the content with a sliding window. Before
 * committing a match at position `idx`, verify that:
 *   - the char immediately before is not a word character
 *     (letter, digit, underscore — anything that would make the
 *     match the middle of a larger word)
 *   - the char immediately after the match is not a word character
 *   - the match is not inside an open `[[...]]` region
 *
 * Matching is case-insensitive (people don't always capitalize page
 * names) but the returned `mentionText` preserves the original case
 * so the UI can show what was actually written.
 */
export function detectMentions(content: string, pageName: string): UnlinkedCandidate[] {
  if (!content || !pageName) return []
  const hits: UnlinkedCandidate[] = []
  const lowerContent = content.toLowerCase()
  const lowerName = pageName.toLowerCase()

  let i = 0
  while (i <= lowerContent.length - lowerName.length) {
    const idx = lowerContent.indexOf(lowerName, i)
    if (idx === -1) break

    const end = idx + lowerName.length
    const beforeChar = idx > 0 ? content[idx - 1] : ''
    const afterChar = end < content.length ? content[end] : ''

    // Word-boundary checks: refuse matches that touch a word
    // character on either side. We allow non-word chars (spaces,
    // punctuation, start/end of string) on the boundaries.
    if (isWordChar(beforeChar) || isWordChar(afterChar)) {
      i = idx + 1
      continue
    }

    // Skip if this mention is inside an open [[ ... ]]
    if (isInsideWikiLink(content, idx)) {
      i = idx + lowerName.length
      continue
    }

    hits.push({
      blockId: '', // filled in by the caller (the caller knows the block)
      pageName,
      mentionText: content.slice(idx, end),
      position: idx,
      createdAt: Date.now(),
    })
    i = end
  }

  return hits
}

/**
 * True when `c` is a letter, digit, or underscore — i.e. a
 * character that can be part of an English word. Anything else
 * (whitespace, punctuation, CJK ideographs that combine with their
 * neighbors) is treated as a word boundary.
 */
function isWordChar(c: string): boolean {
  return /[A-Za-z0-9_]/.test(c)
}

/**
 * Returns true when `position` falls inside an open `[[ ... ]]`
 * region of `content`. We walk back from `position` looking for the
 * nearest `[[` that has not been closed by a `]]` yet.
 */
function isInsideWikiLink(content: string, position: number): boolean {
  // Find the most recent `[[` before `position`
  const openIdx = content.lastIndexOf('[[', position)
  if (openIdx === -1) return false
  // If a `]]` exists between the open and our position, the link is
  // already closed — we're not inside it.
  const closeIdx = content.indexOf(']]', openIdx + 2)
  if (closeIdx !== -1 && closeIdx < position) return false
  return true
}

// ──── Link action ────────────────────────────────────────────────

/**
 * Return a copy of `content` with the mention at `position` wrapped
 * in `[[ ... ]]`. The wrapped text is always the canonical
 * `pageName` (not the matched substring), so a "see demo page" can
 * be promoted to "[[Demo Page]]" without losing the original casing.
 *
 * If the content at `position` is no longer the expected mention
 * (e.g. the block was edited in the meantime), we return the
 * original content unchanged rather than corrupting it. The caller
 * is expected to verify the mutation and refresh the queue.
 */
export function linkifyMention(content: string, candidate: UnlinkedCandidate): string {
  const expected = candidate.mentionText
  const before = content.slice(0, candidate.position)
  const matched = content.slice(candidate.position, candidate.position + expected.length)
  const after = content.slice(candidate.position + expected.length)

  if (matched.toLowerCase() !== expected.toLowerCase()) {
    // The block changed under us — bail out instead of guessing.
    return content
  }
  return `${before}[[${candidate.pageName}]]${after}`
}

// ──── Persistence ────────────────────────────────────────────────

/**
 * Read the persisted queue. Tolerant of:
 *   - missing key  → []
 *   - malformed JSON (e.g. partial write, manual edit) → []
 *   - non-array values → []
 */
export function loadQueue(): UnlinkedCandidate[] {
  if (typeof localStorage === 'undefined') return []
  const raw = localStorage.getItem(STORAGE_KEY)
  if (!raw) return []
  try {
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    // Defensive: only keep entries that look like candidates. A
    // manual edit that puts garbage in localStorage shouldn't crash
    // the panel.
    return parsed.filter(
      (c): c is UnlinkedCandidate =>
        c &&
        typeof c.blockId === 'string' &&
        typeof c.pageName === 'string' &&
        typeof c.mentionText === 'string' &&
        typeof c.position === 'number',
    )
  } catch {
    return []
  }
}

/** Overwrite the persisted queue. */
export function saveQueue(queue: UnlinkedCandidate[]): void {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(STORAGE_KEY, JSON.stringify(queue))
}

/**
 * Drop one candidate from the persisted queue. Two candidates are
 * considered the "same" if they share `blockId` and `position` —
 * the same block can mention the same page in two different places,
 * and dismissing one mention must not silently dismiss the other.
 */
export function removeCandidate(blockId: string, position: number): void {
  const current = loadQueue()
  const next = current.filter((c) => !(c.blockId === blockId && c.position === position))
  saveQueue(next)
}
