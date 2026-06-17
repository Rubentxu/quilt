// ─── rankMainAction.unit.test ───────────────────────────────────────────────

import { describe, it, expect } from 'vitest'
import { rankMainAction, MAIN_ACTION_THRESHOLD, buildMainActionMap } from '../rankMainAction'
import type { RightSidebarSection } from '../types'
import type { BlockSelection, PageSelection } from '../../selection/types'
import type { SectionMainAction } from '../rankMainAction'

// Helper: minimal section stub
function makeSection(id: string, priority: number): RightSidebarSection {
  return {
    id,
    label: id,
    priority,
    visible: true,
    component: () => null,
  }
}

// Helper: main action map
function makeActionMap(actions: Record<string, SectionMainAction>) {
  return new Map(Object.entries(actions))
}

describe('rankMainAction', () => {
  describe('null selection (graph context)', () => {
    it('returns null regardless of sections', () => {
      const sections = [makeSection('s1', 100)]
      const actions = makeActionMap({
        s1: { targetType: 'any', label: 'Action' },
      })
      expect(rankMainAction(null, sections, actions)).toBe(null)
    })
  })

  describe('confidence scoring', () => {
    it('suggestion penalty exactly at threshold (0.7) returns result', () => {
      const sections = [makeSection('s1', 100)]
      const actions = makeActionMap({
        s1: { targetType: 'block', label: 'Block action', suggestion: true },
      })
      // base 1.0 - 0.3 = 0.7, exactly at threshold → NOT null
      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      expect(rankMainAction(blockSel, sections, actions)).not.toBe(null)
    })

    it('confidence 1.0 when type matches exactly (block)', () => {
      const sections = [makeSection('s1', 100)]
      const actions = makeActionMap({
        s1: { targetType: 'block', label: 'Block action' },
      })
      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      const result = rankMainAction(blockSel, sections, actions)
      expect(result?.confidence).toBe(1.0)
      expect(result?.sectionId).toBe('s1')
    })

    it('confidence 1.0 when type matches exactly (page)', () => {
      const sections = [makeSection('s1', 100)]
      const actions = makeActionMap({
        s1: { targetType: 'page', label: 'Page action' },
      })
      const pageSel: PageSelection = { type: 'page', pageName: 'p1', isJournal: false }
      const result = rankMainAction(pageSel, sections, actions)
      expect(result?.confidence).toBe(1.0)
    })

    it('returns null for targetType=any on block selection (0.5 < 0.7 threshold)', () => {
      const sections = [makeSection('s1', 100)]
      const actions = makeActionMap({
        s1: { targetType: 'any', label: 'Any action' },
      })
      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      // 0.5 < 0.7 → null
      const result = rankMainAction(blockSel, sections, actions)
      expect(result).toBe(null)
    })

    it('returns null for type mismatch (page on block, 0.3 < 0.7)', () => {
      const sections = [makeSection('s1', 100)]
      const actions = makeActionMap({
        s1: { targetType: 'page', label: 'Page action' },
      })
      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      // type mismatch: 0.3, below threshold 0.7 → returns null
      const result = rankMainAction(blockSel, sections, actions)
      expect(result).toBe(null)
    })

    it('suggestion penalty of -0.3 applied (exact match = 1.0, minus 0.3 = 0.7)', () => {
      const sections = [makeSection('s1', 100)]
      const actions = makeActionMap({
        s1: { targetType: 'block', label: 'Block suggestion', suggestion: true },
      })
      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      // base 1.0 - 0.3 = 0.7, which exactly meets threshold
      const result = rankMainAction(blockSel, sections, actions)
      expect(result?.confidence).toBe(0.7)
    })
  })

  describe('threshold gate', () => {
    it('returns null when confidence < 0.7', () => {
      const sections = [makeSection('s1', 100)]
      const actions = makeActionMap({
        s1: { targetType: 'page', label: 'Page action' },
      })
      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      // type mismatch: 0.3, below threshold
      expect(rankMainAction(blockSel, sections, actions)).toBe(null)
    })

    it('returns action when confidence >= 0.7', () => {
      const sections = [makeSection('s1', 100)]
      const actions = makeActionMap({
        s1: { targetType: 'block', label: 'Block action' },
      })
      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      // exact match: 1.0, above threshold
      expect(rankMainAction(blockSel, sections, actions)).not.toBe(null)
    })
  })

  describe('tie-breaking', () => {
    it('higher confidence wins', () => {
      const sections = [makeSection('s1', 100), makeSection('s2', 100)]
      const actions = makeActionMap({
        s1: { targetType: 'page', label: 'Page action' },
        s2: { targetType: 'block', label: 'Block action' },
      })
      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      // s2 matches exactly (1.0), s1 mismatches (0.3) → s2 wins
      const result = rankMainAction(blockSel, sections, actions)
      expect(result?.sectionId).toBe('s2')
    })

    it('lower priority wins on equal confidence', () => {
      const sections = [makeSection('s1', 200), makeSection('s2', 100)]
      const actions = makeActionMap({
        s1: { targetType: 'block', label: 'Block action 1' },
        s2: { targetType: 'block', label: 'Block action 2' },
      })
      const blockSel: BlockSelection = { type: 'block', blockId: 'b1', pageName: 'p1' }
      // Both exact match (1.0), s2 has lower priority → s2 wins
      const result = rankMainAction(blockSel, sections, actions)
      expect(result?.sectionId).toBe('s2')
    })
  })

  describe('empty sections / actions', () => {
    it('returns null when no sections', () => {
      expect(rankMainAction({ type: 'block', blockId: 'b1', pageName: 'p1' }, [], makeActionMap({}))).toBe(null)
    })

    it('returns null when section has no action', () => {
      const sections = [makeSection('s1', 100)]
      expect(rankMainAction({ type: 'block', blockId: 'b1', pageName: 'p1' }, sections, makeActionMap({}))).toBe(null)
    })
  })
})

describe('MAIN_ACTION_THRESHOLD', () => {
  it('is 0.7', () => {
    expect(MAIN_ACTION_THRESHOLD).toBe(0.7)
  })
})

describe('buildMainActionMap', () => {
  it('builds map from sections with getMainAction', () => {
    const sections = [makeSection('s1', 100), makeSection('s2', 200)]
    const getMainAction = (id: string): SectionMainAction | undefined => {
      if (id === 's1') return { targetType: 'block', label: 'Action 1' }
      return undefined
    }
    const map = buildMainActionMap(sections, getMainAction)
    expect(map.size).toBe(1)
    expect(map.get('s1')?.label).toBe('Action 1')
    expect(map.get('s2')).toBeUndefined()
  })
})
