import { describe, it, expect } from 'vitest'
import {
  countOpenAnnotations,
  sortByCreatedAtDesc,
  buildAnnotationThread,
} from '../annotationUtils'
import type { Annotation } from '@shared/types/api'

function ann(overrides: Partial<Annotation> & { id: string; createdAt: string }): Annotation {
  return {
    id: overrides.id,
    blockId: 'b1',
    scope: 'block',
    authorType: 'human',
    authorName: 'alice',
    content: 'x',
    status: overrides.status ?? 'pending',
    createdAt: overrides.createdAt,
    ...overrides,
  } as Annotation
}

describe('countOpenAnnotations', () => {
  it('counts pending + in_progress, ignores resolved + dismissed', () => {
    const list: Annotation[] = [
      ann({ id: '1', createdAt: '2026-01-01', status: 'pending' }),
      ann({ id: '2', createdAt: '2026-01-02', status: 'in_progress' }),
      ann({ id: '3', createdAt: '2026-01-03', status: 'resolved' }),
      ann({ id: '4', createdAt: '2026-01-04', status: 'dismissed' }),
    ]
    expect(countOpenAnnotations(list)).toBe(2)
  })

  it('returns 0 for an empty list', () => {
    expect(countOpenAnnotations([])).toBe(0)
  })
})

describe('sortByCreatedAtDesc', () => {
  it('sorts by createdAt DESC, tie-break by id DESC', () => {
    const list = [
      ann({ id: 'a', createdAt: '2026-01-01' }),
      ann({ id: 'b', createdAt: '2026-01-03' }),
      ann({ id: 'c', createdAt: '2026-01-01' }),
    ]
    const sorted = sortByCreatedAtDesc(list)
    expect(sorted.map(a => a.id)).toEqual(['b', 'c', 'a'])
  })

  it('does not mutate the input array', () => {
    const list = [
      ann({ id: 'a', createdAt: '2026-01-02' }),
      ann({ id: 'b', createdAt: '2026-01-01' }),
    ]
    const copyBefore = list.map(a => a.id)
    sortByCreatedAtDesc(list)
    expect(list.map(a => a.id)).toEqual(copyBefore)
  })
})

describe('buildAnnotationThread', () => {
  it('returns roots with nested replies', () => {
    const list: Annotation[] = [
      ann({ id: 'r1', createdAt: '2026-01-01' }),
      ann({ id: 'r2', createdAt: '2026-01-02', parentAnnotationId: 'r1' }),
      ann({ id: 'r3', createdAt: '2026-01-03', parentAnnotationId: 'r2' }),
      ann({ id: 'r4', createdAt: '2026-01-04' }),
    ]
    const tree = buildAnnotationThread(list)
    expect(tree).toHaveLength(2)
    expect(tree[0].annotation.id).toBe('r1')
    expect(tree[0].replies).toHaveLength(1)
    expect(tree[0].replies[0].annotation.id).toBe('r2')
    expect(tree[0].replies[0].replies).toHaveLength(1)
    expect(tree[1].annotation.id).toBe('r4')
    expect(tree[1].replies).toHaveLength(0)
  })

  it('treats orphan replies (parent missing) as roots', () => {
    const list: Annotation[] = [
      ann({ id: 'r1', createdAt: '2026-01-01' }),
      ann({ id: 'orphan', createdAt: '2026-01-02', parentAnnotationId: 'missing' }),
    ]
    const tree = buildAnnotationThread(list)
    expect(tree).toHaveLength(2)
    const ids = tree.map(n => n.annotation.id)
    expect(ids).toContain('orphan')
  })
})
