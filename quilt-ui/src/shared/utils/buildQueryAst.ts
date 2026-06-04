/**
 * buildQueryAst — build a QueryAst from FilterChip[].
 *
 * F18: Converts the UI's filter chip state into a QueryAst that can be
 * sent to `POST /api/v1/query`.
 *
 * - Single chip → returns the Property AST directly
 * - Multiple chips → AND-combines them
 * - Empty chips → returns null (no filter)
 */

import type { FilterChip } from '@shared/types/filterChip';
import type { QueryAst, QueryValue } from '@shared/types/queryAst';

/**
 * Convert a FilterChip to a QueryAst.
 * Single chip = property AST directly (no And wrapper).
 */
function chipToAst(chip: FilterChip): QueryAst {
  const value: QueryValue = { String: chip.value };
  const value2: QueryValue | undefined = chip.value2
    ? { String: chip.value2 }
    : undefined;

  return {
    Property: {
      key: chip.key,
      op: chip.op,
      value,
      value2,
    },
  };
}

/**
 * Build a QueryAst from an ordered list of FilterChips.
 *
 * - Empty list → null (no filtering)
 * - Single chip → the chip's Property AST directly
 * - Multiple chips → AND-combined Property ASTs
 */
export function buildQueryAst(chips: FilterChip[]): QueryAst | null {
  if (chips.length === 0) {
    return null;
  }

  if (chips.length === 1) {
    return chipToAst(chips[0]);
  }

  // Multiple chips → AND combine
  return {
    And: chips.map(chipToAst),
  };
}
