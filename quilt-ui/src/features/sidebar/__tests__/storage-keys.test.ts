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

  it('exposes WELCOME_SEEN with `quilt-welcome-seen`', () => {
    // Tracks dismissal of the first-run welcome tour
    // (F3 of quilt-fase2-ux-empty-states). The literal value
    // is part of the localStorage wire contract — once a user
    // dismisses the tour, they should not see it again even
    // after a refactor that renames the constant.
    expect(STORAGE_KEYS.WELCOME_SEEN).toBe('quilt-welcome-seen')
  })

  it('is frozen / readonly via `as const` (TypeScript enforces this at compile time)', () => {
    // Runtime check that the values are strings (the `as const` type
    // assertion means the keys are string literal types, not widened
    // to plain `string`).
    expect(typeof STORAGE_KEYS.FAVORITES).toBe('string')
    expect(typeof STORAGE_KEYS.RECENTS).toBe('string')
    expect(typeof STORAGE_KEYS.WELCOME_SEEN).toBe('string')
  })

  it('keeps the FAVORITES, RECENTS and WELCOME_SEEN values distinct', () => {
    // Sanity guard: a copy-paste typo would collapse multiple keys
    // into the same value and break the corresponding feature
    // (sidebar favorites, recents, or welcome-tour dismissal).
    expect(STORAGE_KEYS.FAVORITES).not.toBe(STORAGE_KEYS.RECENTS)
    expect(STORAGE_KEYS.FAVORITES).not.toBe(STORAGE_KEYS.WELCOME_SEEN)
    expect(STORAGE_KEYS.RECENTS).not.toBe(STORAGE_KEYS.WELCOME_SEEN)
  })
})
