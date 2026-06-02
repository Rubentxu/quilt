/**
 * E2E Auth State — Bearer token provider for Playwright specs.
 *
 * Reads `QUILT_API_KEY` from the environment (same env var the server uses).
 * Every API call from E2E specs must include the Authorization header.
 *
 * Server auth model (middleware/auth.rs):
 *   - /api/v1/* → requires `Authorization: Bearer <key>`
 *   - /health, /metrics, /ws, /, /*path → public
 *
 * Usage:
 *   import { getAuthHeaders, requireApiKey } from '../auth-state';
 *   const headers = getAuthHeaders();
 *   await page.request.post('/api/v1/blocks', { data: {...}, headers });
 */

/** Throws if QUILT_API_KEY is not set. Call at the top of any spec that needs auth. */
export function requireApiKey(): string {
  const key = process.env.QUILT_API_KEY;
  if (!key) {
    throw new Error(
      'QUILT_API_KEY env var is required for E2E tests. ' +
      'The server prints it on startup or reads it from QUILT_API_KEY env. ' +
      'Set it before running: QUILT_API_KEY=<your-key> npx playwright test'
    );
  }
  return key;
}

/** Returns Authorization headers for API calls. */
export function getAuthHeaders(): Record<string, string> {
  return { Authorization: `Bearer ${requireApiKey()}` };
}
