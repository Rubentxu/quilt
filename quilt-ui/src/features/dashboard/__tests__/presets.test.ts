// ─── presets.test.ts — DashboardLayout panel presets ─────────────
//
// The presets are the *visible-by-default* panel sets that the
// DashboardLayout exposes. They are pure data — no React, no IO —
// so the test surface is small: each preset is a `Set<PanelId>`
// and `getPreset` resolves by name.

import { describe, it, expect } from 'vitest'
import { PRESETS, getPreset, type PresetId } from '../presets'

describe('presets — DashboardLayout', () => {
  it('exposes exactly three named presets (default, focus, review)', () => {
    // Adding a preset is a user-facing change (it shows up in the
    // LayoutMenu). Force a conscious decision by pinning the count.
    expect(Object.keys(PRESETS)).toHaveLength(3)
    expect(Object.keys(PRESETS).sort()).toEqual(['default', 'focus', 'review'])
  })

  it('default preset includes the sidebar and backlinks panels', () => {
    const set = getPreset('default')
    expect(set.has('sidebar')).toBe(true)
    expect(set.has('backlinks')).toBe(true)
  })

  it('focus preset hides the sidebar (full-width writing surface)', () => {
    const set = getPreset('focus')
    expect(set.has('sidebar')).toBe(false)
  })

  it('review preset includes sidebar, backlinks, and agent activity', () => {
    const set = getPreset('review')
    expect(set.has('sidebar')).toBe(true)
    expect(set.has('backlinks')).toBe(true)
    expect(set.has('agent-activity')).toBe(true)
  })

  it('focus preset is a proper subset of the default preset', () => {
    // The 'focus' preset is the *minimal* one — every panel in it
    // must also exist in 'default'. This catches accidental
    // "orphan" panel additions.
    const focus = getPreset('focus')
    const def = getPreset('default')
    for (const panel of focus) {
      expect(def.has(panel)).toBe(true)
    }
  })

  it('getPreset returns a Set instance (not an array) — Set membership is part of the contract', () => {
    const set = getPreset('default')
    expect(set).toBeInstanceOf(Set)
  })

  it('getPreset returns a fresh Set every call (mutating one does not poison the preset table)', () => {
    const a = getPreset('default')
    const b = getPreset('default')
    expect(a).not.toBe(b)
    a.add('ghost-panel')
    expect(b.has('ghost-panel')).toBe(false)
  })

  it('getPreset falls back to the default preset for an unknown id', () => {
    // A stale `PresetId` from localStorage must not crash the
    // provider; we degrade to the default preset gracefully.
    const set = getPreset('nope' as unknown as PresetId)
    expect(set).toEqual(getPreset('default'))
  })
})
