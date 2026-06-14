/**
 * Typed Operators — P2.5 Batch 10.
 *
 * Maps each `PropertyType` (from `propertySchema.ts`) to the set of
 * filter operators that are semantically valid for that type.
 *
 * The operators used here come from TWO sources:
 *   - `PropertyOp` (from `queryAst.ts`): the 8 variants the query
 *     engine understands — Equals, NotEquals, Contains, GreaterThan,
 *     LessThan, GreaterThanOrEqual, LessThanOrEqual, Between.
 *   - `FilterOperator` (from `viewConfig.ts`): extends PropertyOp
 *     with two emptiness checks (IsEmpty, IsNotEmpty) and two date
 *     comparators (Before, After).
 *
 * FilterChip.op is typed as `PropertyOp` (we are NOT changing that
 * contract in this batch), so a chip that ends up with an
 * IsEmpty/IsNotEmpty/Before/After op is a runtime-only "lie" against
 * the type — exactly the same situation SavedViewBlock had to accept
 * when it cast `c.operator as any` in the FilterOperator → FilterChip
 * bridging code. The dropdown in FilterChipGroup uses `as PropertyOp`
 * for the same reason.
 *
 * When `propertyTypes` is NOT provided to FilterChipGroup, the full
 * PropertyOp list is shown (V1 backward compatibility). When it IS
 * provided, the dropdown is filtered to the operators valid for the
 * selected key's type.
 */

import type { PropertyType } from './propertySchema';

/** Operators available for each property type. */
export const OPERATORS_BY_TYPE: Record<PropertyType, readonly string[]> = {
  text:         ['Equals', 'NotEquals', 'Contains', 'IsEmpty', 'IsNotEmpty'],
  number:       ['Equals', 'NotEquals', 'GreaterThan', 'LessThan', 'GreaterThanOrEqual', 'LessThanOrEqual', 'IsEmpty'],
  select:       ['Equals', 'NotEquals', 'IsEmpty'],
  multi_select: ['Equals', 'Contains', 'IsEmpty'],
  date:         ['Equals', 'Before', 'After', 'IsEmpty'],
  boolean:      ['Equals'],
  url:          ['Equals', 'NotEquals', 'Contains', 'IsEmpty', 'IsNotEmpty'],
  person:       ['Equals', 'IsEmpty'],
  relation:     ['Equals', 'IsEmpty'],
  file:         ['Equals', 'Contains', 'IsEmpty'],
};

/**
 * Get the list of valid operators for a given property type.
 *
 * Falls back to a small default (Equals / NotEquals / Contains) when
 * the type is unknown — e.g. when the schema is missing or the type
 * string is malformed. This keeps the dropdown functional instead of
 * crashing the popover on bad data.
 */
export function getOperatorsForType(type: string): readonly string[] {
  return (OPERATORS_BY_TYPE as Record<string, readonly string[]>)[type]
    ?? ['Equals', 'NotEquals', 'Contains'];
}
