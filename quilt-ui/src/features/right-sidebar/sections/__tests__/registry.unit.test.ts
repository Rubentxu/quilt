// ─── registry.unit.test ─────────────────────────────────────────────────────

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import {
  registerSection,
  getSections,
  getVisibleSections,
  isRegistered,
  _resetForTesting,
  createSectionRegistry,
} from '../registry'
import type { RightSidebarSection } from '../types'
import type { BlockSelection } from '../../selection/types'

function makeSection(id: string, priority: number, visible = true): RightSidebarSection {
  return { id, label: id, priority, visible, component: () => null }
}

describe('registry', () => {
  beforeEach(() => {
    _resetForTesting()
  })
  afterEach(() => {
    _resetForTesting()
  })

  describe('registerSection', () => {
    it('registers a section', () => {
      registerSection(makeSection('s1', 100))
      expect(getSections().map((s) => s.id)).toEqual(['s1'])
    })

    it('registers multiple sections sorted by priority', () => {
      registerSection(makeSection('s3', 300))
      registerSection(makeSection('s1', 100))
      registerSection(makeSection('s2', 200))
      expect(getSections().map((s) => s.id)).toEqual(['s1', 's2', 's3'])
    })

    it('overwrites existing section with same id', () => {
      registerSection(makeSection('s1', 100))
      registerSection({ ...makeSection('s1', 200), label: 'Updated' })
      const sections = getSections()
      expect(sections.length).toBe(1)
      expect(sections[0].priority).toBe(200)
      expect(sections[0].label).toBe('Updated')
    })

    it('sets isRegistered to true after first registration', () => {
      expect(isRegistered()).toBe(false)
      registerSection(makeSection('s1', 100))
      expect(isRegistered()).toBe(true)
    })
  })

  describe('getVisibleSections', () => {
    it('filters by visible flag', () => {
      registerSection(makeSection('s1', 100, true))
      registerSection(makeSection('s2', 200, false))
      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      expect(getVisibleSections(blockSel).map((s) => s.id)).toEqual(['s1'])
    })

    it('filters by predicate', () => {
      registerSection({
        ...makeSection('s1', 100),
        predicate: (sel) => sel?.type === 'block',
      })
      registerSection({
        ...makeSection('s2', 200),
        predicate: (sel) => sel?.type === 'page',
      })
      registerSection(makeSection('s3', 300))

      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      expect(getVisibleSections(blockSel).map((s) => s.id)).toEqual(['s1', 's3'])
    })

    it('combines visible flag and predicate', () => {
      registerSection({
        ...makeSection('s1', 100, true),
        predicate: (sel) => sel?.type === 'block',
      })
      registerSection({
        ...makeSection('s2', 200, false),
        predicate: (sel) => sel?.type === 'block',
      })
      registerSection({
        ...makeSection('s3', 300, true),
        predicate: (sel) => sel?.type === 'page',
      })

      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      expect(getVisibleSections(blockSel).map((s) => s.id)).toEqual(['s1'])
    })
  })
})

describe('createSectionRegistry (isolated instance)', () => {
  it('creates an isolated registry', () => {
    const registry = createSectionRegistry()
    registry.registerSection(makeSection('isolated-s1', 100))
    // Global registry should be unaffected
    expect(getSections().map((s) => s.id)).not.toContain('isolated-s1')
    // Isolated registry should have it
    expect(registry.getSections().map((s) => s.id)).toContain('isolated-s1')
  })

  it('isolated getVisibleSections works independently', () => {
    const registry = createSectionRegistry()
    registry.registerSection({
      ...makeSection('s1', 100),
      predicate: (sel) => sel?.type === 'block',
    })
    const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
    expect(registry.getVisibleSections(blockSel).map((s) => s.id)).toEqual(['s1'])
  })

  it('_reset clears the isolated registry', () => {
    const registry = createSectionRegistry()
    registry.registerSection(makeSection('s1', 100))
    registry._reset()
    expect(registry.getSections()).toHaveLength(0)
  })
})
