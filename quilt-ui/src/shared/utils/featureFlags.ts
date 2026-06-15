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

/**
 * Feature flag utility for the projection renderer change.
 *
 * The `VITE_PROJECTION_RENDERER` flag toggles between the legacy
 * BlockRow rendering and the new ProjectionRenderer component.
 * When ON (default), BlockRow delegates rendering to ProjectionRenderer.
 * When OFF, the legacy rendering path is preserved for safe rollback.
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

/** Raw env-var read for projection flag — exported only for tests. */
export function parseProjectionFlag(raw: string | undefined): boolean {
  if (raw === undefined) return false // Default to OFF for backward compatibility
  const normalized = raw.trim().toLowerCase()
  if (normalized === '') return false
  if (TRUTHY.has(normalized)) return true
  if (['false', '0', 'no', 'off'].includes(normalized)) return false
  return false
}

let _enabled: boolean = parseAnnotationFlag(
  // Vite injects `import.meta.env.VITE_*` at build/dev time; undefined
  // in unit tests. The `typeof === 'string'` guard keeps the strict
  // tsc build happy when the env type is not augmented.
  typeof import.meta.env.VITE_QUILT_ANNOTATIONS_ENABLED === 'string'
    ? import.meta.env.VITE_QUILT_ANNOTATIONS_ENABLED
    : undefined,
)

let _projectionEnabled: boolean = parseProjectionFlag(
  typeof import.meta.env.VITE_PROJECTION_RENDERER === 'string'
    ? import.meta.env.VITE_PROJECTION_RENDERER
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
 * Returns true when the frontend should use the new ProjectionRenderer
 * instead of the legacy BlockRow rendering.
 */
export function isProjectionRendererEnabled(): boolean {
  return _projectionEnabled
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

/**
 * Override the projection renderer flag value. **Test-only**
 */
export function __setProjectionRendererEnabledForTest(value: boolean): () => void {
  const previous = _projectionEnabled
  _projectionEnabled = value
  return () => {
    _projectionEnabled = previous
  }
}

// `vi` is re-exported here so test files don't have to import both
// `vitest` AND this module to stub + restore. This is the same shape
// the React Testing Library community uses for utility shims.
export { vi }
