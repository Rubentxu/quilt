import { describe, it, expect } from 'vitest'
import { STORAGE_KEYS } from '../storage-keys'

// Tests for the central localStorage key namespace.
//
// All sidebar-owned localStorage keys are namespaced `quilt-*` per
// the (informal) convention used by `quilt-favorites`. The
// `quilt-recents` key is being introduced for the upcoming recent
// pages tracking (PR 2). Centralising the keys here prevents typos
// and gives one place to audit what the sidebar persists.

describe('STORAGE_KEYS', () => {
  it('exposes FAVORITES with the legacy `quilt-favorites` value', () => {
    // The literal value is part of the public wire contract — users
    // already have entries in `localStorage` from before this
    // constant existed. Changing the value would orphan their data.
    expect(STORAGE_KEYS.FAVORITES).toBe('quilt-favorites')
  })

  it('exposes RECENTS with `quilt-recents`', () => {
    expect(STORAGE_KEYS.RECENTS).toBe('quilt-recents')
  })

  it('is frozen / readonly via `as const` (TypeScript enforces this at compile time)', () => {
    // Runtime check that the values are strings (the `as const` type
    // assertion means the keys are string literal types, not widened
    // to plain `string`).
    expect(typeof STORAGE_KEYS.FAVORITES).toBe('string')
    expect(typeof STORAGE_KEYS.RECENTS).toBe('string')
  })

  it('keeps the FAVORITES and RECENTS values distinct', () => {
    // Sanity guard: a copy-paste typo would collapse both into the
    // same key and break the sidebar.
    expect(STORAGE_KEYS.FAVORITES).not.toBe(STORAGE_KEYS.RECENTS)
  })
})
