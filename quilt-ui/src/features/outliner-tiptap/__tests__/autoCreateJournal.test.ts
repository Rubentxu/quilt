/**
 * Test the auto-create-on-empty-journal flow.
 *
 * This is a pure logic test — it verifies the `autoCreatedRef` Set is
 * used correctly so we don't create duplicate blocks on re-renders or
 * StrictMode double-invocation.
 */

import { describe, it, expect } from 'vitest'

// The auto-create guard is a simple Set check. Replicate the relevant
// logic here so we can test it in isolation.
function shouldAutoCreate(
  pageName: string,
  isJournal: boolean,
  fetchedBlockCount: number,
  alreadyCreated: Set<string>,
): boolean {
  return (
    isJournal &&
    fetchedBlockCount === 0 &&
    !alreadyCreated.has(pageName)
  )
}

function markCreated(pageName: string, set: Set<string>): void {
  set.add(pageName)
}

describe('Journal auto-create guard', () => {
  it('auto-creates for an empty journal', () => {
    const set = new Set<string>()
    expect(shouldAutoCreate('2026-06-02', true, 0, set)).toBe(true)
  })

  it('does NOT auto-create for a non-journal page', () => {
    const set = new Set<string>()
    expect(shouldAutoCreate('notes', false, 0, set)).toBe(false)
  })

  it('does NOT auto-create when the journal has blocks', () => {
    const set = new Set<string>()
    expect(shouldAutoCreate('2026-06-02', true, 3, set)).toBe(false)
  })

  it('does NOT auto-create twice for the same page', () => {
    const set = new Set<string>()
    expect(shouldAutoCreate('2026-06-02', true, 0, set)).toBe(true)
    markCreated('2026-06-02', set)
    expect(shouldAutoCreate('2026-06-02', true, 0, set)).toBe(false)
  })

  it('auto-creates for different journal pages independently', () => {
    const set = new Set<string>()
    expect(shouldAutoCreate('2026-06-02', true, 0, set)).toBe(true)
    markCreated('2026-06-02', set)
    expect(shouldAutoCreate('2026-06-01', true, 0, set)).toBe(true)
    expect(shouldAutoCreate('2026-05-31', true, 0, set)).toBe(true)
  })
})
