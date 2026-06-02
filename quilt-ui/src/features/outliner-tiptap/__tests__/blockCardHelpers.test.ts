import { describe, it, expect } from 'vitest'
import { getBlockType, getBlockMetas } from '../PageView'

describe('getBlockType', () => {
  it('returns null when block has no properties', () => {
    expect(getBlockType({})).toBeNull()
  })

  it('returns null when properties is empty', () => {
    expect(getBlockType({ properties: [] })).toBeNull()
  })

  it('returns null when type is a plain string', () => {
    const block = {
      properties: [{ key: 'type', value: 'paragraph', type: 'string' as const }],
    }
    expect(getBlockType(block)).toBeNull()
  })

  it('returns reference when type:: reference', () => {
    const block = {
      properties: [{ key: 'type', value: 'reference', type: 'string' as const }],
    }
    expect(getBlockType(block)).toBe('reference')
  })

  it('returns documentacion when type:: documentacion', () => {
    const block = {
      properties: [{ key: 'type', value: 'documentacion', type: 'string' as const }],
    }
    expect(getBlockType(block)).toBe('documentacion')
  })

  it('ignores other properties', () => {
    const block = {
      properties: [
        { key: 'dda-relacionada', value: 'DDA v1', type: 'string' as const },
        { key: 'type', value: 'reference', type: 'string' as const },
      ],
    }
    expect(getBlockType(block)).toBe('reference')
  })
})

describe('getBlockMetas', () => {
  it('returns empty array when block has no properties', () => {
    expect(getBlockMetas({})).toEqual([])
  })

  it('excludes the type property (discriminator)', () => {
    const block = {
      properties: [
        { key: 'type', value: 'reference', type: 'string' as const },
        { key: 'dda-relacionada', value: 'DDA v1', type: 'string' as const },
      ],
    }
    expect(getBlockMetas(block)).toEqual([{ key: 'dda-relacionada', value: 'DDA v1' }])
  })

  it('maps all non-type properties', () => {
    const block = {
      properties: [
        { key: 'type', value: 'documentacion', type: 'string' as const },
        { key: 'fecha-creacion', value: '26-05-2026', type: 'string' as const },
        { key: 'author', value: 'claude', type: 'string' as const },
      ],
    }
    expect(getBlockMetas(block)).toEqual([
      { key: 'fecha-creacion', value: '26-05-2026' },
      { key: 'author', value: 'claude' },
    ])
  })

  it('converts non-string values to string', () => {
    const block = {
      properties: [
        { key: 'priority', value: 1, type: 'number' as const },
        { key: 'resolved', value: false, type: 'boolean' as const },
      ],
    }
    expect(getBlockMetas(block)).toEqual([
      { key: 'priority', value: '1' },
      { key: 'resolved', value: 'false' },
    ])
  })
})
