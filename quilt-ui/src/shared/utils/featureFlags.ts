/**
 * Feature flag utility for the annotations-comments-unification
 * change.
 *
 * The `QUILT_ANNOTATIONS_ENABLED` flag toggles between the legacy
 * block-property comment path (`type: "comment"` blocks) and the
 * new annotation API (`/api/v1/annotations`). When the flag is ON
 * (default), the frontend reads/writes through the new annotation
 * REST surface. When OFF, the legacy path is preserved so a
 * regression on the new path can be rolled back without losing
 * existing comment data.
 *
 * Reading the flag:
 *   - The value is read from `VITE_QUILT_ANNOTATIONS_ENABLED` at
 *     module-load time (matches the existing `VITE_QUILT_API_KEY`
 *     pattern in `api-client.ts`).
 *   - Truthy values: `"true"`, `"1"`, `"yes"`, `"on"` (case-insensitive)
 *   - Falsy values:  `"false"`, `"0"`, `"no"`, `"off"`, `""`, `undefined`
 *   - Anything else: treated as truthy to err on the side of the
 *     new behaviour (it's the path we want to validate in dev).
 *
 * The default is TRUE — by the time this change is fully merged the
 * backend is the only path forward; the flag exists to let
 * production toggles flip back to block-property comments without a
 * deploy if a regression is reported. New code paths should call
 * `isAnnotationsEnabled()` rather than reading the env var
 * directly so tests can stub the function.
 */

import { vi } from 'vitest'

const TRUTHY = new Set(['true', '1', 'yes', 'on'])

/** Raw env-var read — exported only for tests. */
export function parseAnnotationFlag(raw: string | undefined): boolean {
  if (raw === undefined) return true
  const normalized = raw.trim().toLowerCase()
  if (normalized === '') return true
  if (TRUTHY.has(normalized)) return true
  if (['false', '0', 'no', 'off'].includes(normalized)) return false
  // Unknown string — treat as truthy (dev-friendly default).
  return true
}

let _enabled: boolean = parseAnnotationFlag(
  // Vite injects `import.meta.env.VITE_*` at build/dev time; undefined
  // in unit tests. The `typeof === 'string'` guard keeps the strict
  // tsc build happy when the env type is not augmented.
  typeof import.meta.env.VITE_QUILT_ANNOTATIONS_ENABLED === 'string'
    ? import.meta.env.VITE_QUILT_ANNOTATIONS_ENABLED
    : undefined,
)

/**
 * Returns true when the frontend should use the new annotation
 * REST surface instead of the block-property comment path.
 */
export function isAnnotationsEnabled(): boolean {
  return _enabled
}

/**
 * Override the in-memory flag value. **Test-only** — production code
 * should never call this; the flag is read from the env at boot.
 *
 * Returns a restore function for symmetry with vi.stubEnv patterns.
 */
export function __setAnnotationsEnabledForTest(value: boolean): () => void {
  const previous = _enabled
  _enabled = value
  return () => {
    _enabled = previous
  }
}

// `vi` is re-exported here so test files don't have to import both
// `vitest` AND this module to stub + restore. This is the same shape
// the React Testing Library community uses for utility shims.
export { vi }
