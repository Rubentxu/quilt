/**
 * validateQuery — client-side DSL validation via WASM.
 *
 * Uses the quilt-core WASM `query_validate` export when available,
 * falling back to a no-op that always returns valid.
 *
 * Returns:
 *   { valid: true, ast: QueryAst, error: null }  — on success
 *   { valid: false, ast: null, error: string }   — on failure
 */

import type { QueryAst } from '@shared/types/queryAst'

interface ValidateResult {
  valid: boolean
  ast: QueryAst | null
  error: string | null
}

/**
 * Validate a DSL query string using the WASM module.
 *
 * Falls back to always-valid if WASM is not loaded / export missing.
 */
export async function validateQuery(dsl: string): Promise<ValidateResult> {
  if (!dsl.trim()) {
    return { valid: false, ast: null, error: 'Empty query' }
  }

  try {
    // Dynamic import to avoid hard coupling to the WASM bundle
    const mod = await import('@core/wasm-bridge/wasm-loader')
    // query_validate was added to wasm.rs but may not be in the built pkg yet.
    // Use getExport to look it up at runtime.
    const fn = (mod as Record<string, unknown>).query_validate as
      | ((q: string) => unknown)
      | undefined

    if (typeof fn !== 'function') {
      // WASM export not available — skip client validation
      return { valid: true, ast: null, error: null }
    }

    const raw = fn(dsl)
    const jsonStr = typeof raw === 'string' ? raw : JSON.stringify(raw)
    const parsed = JSON.parse(jsonStr) as {
      valid: boolean
      error: string | null
      ast: QueryAst | null
    }

    return {
      valid: parsed.valid ?? false,
      ast: parsed.ast ?? null,
      error: parsed.error ?? (parsed.valid ? null : 'Unknown error'),
    }
  } catch {
    // WASM validation failed — caller will validate via server
    return { valid: true, ast: null, error: null }
  }
}
