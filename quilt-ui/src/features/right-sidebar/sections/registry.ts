// ─── sections/registry — RightSidebarSection registry ───────────────────────
//
// Leaf module — does NOT import feature panels. Only the barrel
// (sections/index.ts) imports features to register them.
//
// ## Design
// The registry uses a module-level singleton for the live instance
// and a factory function for test isolation (createSectionRegistry).
//
// ## Registration
// Features register via side-effect barrel imports in sections/index.ts.
// Each feature calls `registerSection(...)` which is why sections/index.ts
// must be a pure re-export module (no logic) — if it had logic and
// imported panels, it would create an import cycle.

import type { RightSidebarSection, SectionPriority } from './types'

// ─── Singleton registry state ───────────────────────────────────────────────

let _sections: RightSidebarSection[] = []
let _registered = false

// ─── API ───────────────────────────────────────────────────────────────────

/**
 * Register a section. Called by feature barrel imports as a side effect.
 * Idempotent — re-registering the same id overwrites the previous entry.
 */
export function registerSection(section: RightSidebarSection): void {
  const idx = _sections.findIndex((s) => s.id === section.id)
  if (idx >= 0) {
    _sections = [..._sections.slice(0, idx), section, ..._sections.slice(idx + 1)]
  } else {
    _sections = [..._sections, section]
    // Mark registration complete so tests can detect whether
    // feature barrels have been imported.
    _registered = true
  }
}

/**
 * Returns all registered sections, sorted by priority asc then reg order.
 */
export function getSections(): readonly RightSidebarSection[] {
  return [..._sections].sort((a, b) => {
    if (a.priority !== b.priority) return a.priority - b.priority
    // Stable sort by registration index: earlier registration wins
    return 0 // sections were appended in registration order
  })
}

/**
 * Returns sections filtered by predicate + visibility.
 */
export function getVisibleSections(selection: import('../selection/types').Selection): readonly RightSidebarSection[] {
  return getSections().filter((s) => {
    if (!s.visible) return false
    if (s.predicate && !s.predicate(selection)) return false
    return true
  })
}

/**
 * True when at least one section has been registered.
 * Used by tests to verify barrel imports ran.
 */
export function isRegistered(): boolean {
  return _registered
}

/**
 * Reset the registry to empty. FOR TESTS ONLY — do not call in production.
 */
export function _resetForTesting(): void {
  _sections = []
  _registered = false
}

/**
 * Factory for creating an isolated registry instance.
 * Tests can pass a custom registry to a component instead of relying
 * on the module singleton.
 *
 * Returns a registry object with the same interface as the module exports
 * but operating on an independent internal array.
 */
export function createSectionRegistry(): {
  registerSection: (section: RightSidebarSection) => void
  getSections: () => readonly RightSidebarSection[]
  getVisibleSections: (selection: import('../selection/types').Selection) => readonly RightSidebarSection[]
  _reset: () => void
} {
  let sections: RightSidebarSection[] = []

  return {
    registerSection(section: RightSidebarSection) {
      const idx = sections.findIndex((s) => s.id === section.id)
      if (idx >= 0) {
        sections = [...sections.slice(0, idx), section, ...sections.slice(idx + 1)]
      } else {
        sections = [...sections, section]
      }
    },
    getSections() {
      return [...sections].sort((a, b) => a.priority - b.priority)
    },
    getVisibleSections(selection) {
      return this.getSections().filter((s) => {
        if (!s.visible) return false
        if (s.predicate && !s.predicate(selection)) return false
        return true
      })
    },
    _reset() {
      sections = []
    },
  }
}
