/**
 * useKanbanGrouping — F22 Kanban View
 *
 * Hook for extracting Kanban grouping information from blocks.
 * Determines which property to use for grouping and provides
 * the grouping logic.
 */

import { useMemo } from 'react'
import type { Block } from '@shared/types/api'

export interface KanbanGroupingResult {
  /** Property key to group by */
  propertyKey: string
  /** Whether a valid grouping property was found */
  isValid: boolean
  /** All unique values for the grouping property */
  columnValues: string[]
}

/**
 * Extracts the property to use for Kanban grouping.
 * Uses the first property that has multiple distinct values.
 */
export function useKanbanGrouping(blocks: Block[]): KanbanGroupingResult {
  return useMemo(() => {
    if (blocks.length === 0) {
      return { propertyKey: '', isValid: false, columnValues: [] }
    }

    // Count property occurrences
    const propertyCounts = new Map<string, Map<string, number>>()

    for (const block of blocks) {
      if (!block.properties) continue

      for (const prop of block.properties) {
        if (prop.key === 'template') continue // Skip template property

        if (!propertyCounts.has(prop.key)) {
          propertyCounts.set(prop.key, new Map())
        }
        const value = String(prop.value)
        const count = propertyCounts.get(prop.key)!.get(value) ?? 0
        propertyCounts.get(prop.key)!.set(value, count + 1)
      }
    }

    // Find property with most distinct values (min 2)
    let bestProperty = ''
    let maxDistinct = 0

    for (const [key, values] of propertyCounts) {
      const distinctCount = values.size
      if (distinctCount > maxDistinct) {
        maxDistinct = distinctCount
        bestProperty = key
      }
    }

    if (maxDistinct < 2) {
      return { propertyKey: '', isValid: false, columnValues: [] }
    }

    // Get all unique values for the best property
    const columnValues = new Set<string>()
    for (const block of blocks) {
      const prop = block.properties?.find(p => p.key === bestProperty)
      if (prop) {
        columnValues.add(String(prop.value))
      }
    }

    return {
      propertyKey: bestProperty,
      isValid: true,
      columnValues: Array.from(columnValues).sort(),
    }
  }, [blocks])
}
