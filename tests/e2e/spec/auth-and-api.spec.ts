/**
 * Auth & API E2E Tests
 *
 * Verifies the auth boundary (Bearer token enforcement) and the
 * home → today's journal redirect.
 *
 * Notes on server routes used here:
 *   - GET    /api/v1/blocks              → 200 with auth, 401 without
 *   - GET    /api/v1/blocks/:id          → 405 (no GET handler on that path)
 *   - DELETE /api/v1/blocks/:id          → 404 when the uuid doesn't exist
 *   - GET    /health                     → 200 { status: "ok" }
 *
 * Test #8 uses DELETE because that's the only verb that actually
 * returns 404 for a missing block uuid — GET is 405 (Method Not Allowed),
 * not 404. See `crates/quilt-server/src/handlers/blocks.rs` for the route
 * table.
 *
 * Prerequisites:
 *   - Server running on localhost:3737
 *   - Frontend running on localhost:5173 (Playwright will start it via
 *     `webServer` in playwright.config.ts)
 *   - QUILT_API_KEY env var set
 *
 * Run with: QUILT_API_KEY=<key> npx playwright test auth-and-api
 */

import { test, expect } from '@playwright/test';
import { getAuthHeaders, requireApiKey } from '../auth-state';

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737';
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173';

// Sanity check — fail fast at file load if the key is missing.
requireApiKey();

/** Today's date in YYYY-MM-DD using LOCAL time (matches HomePage.tsx). */
function todayLocalDate(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, '0');
  const d = String(now.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

// ─── Test group: Home Redirect ──────────────────────────────────────────────

test.describe('Home Redirect', () => {
  test('@smoke root path redirects to graph selection', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/`);

    // The app redirects to /select-graph when no graph is selected
    await page.waitForURL(/\/select-graph/, { timeout: 15_000 });
    expect(page.url()).toContain('/select-graph');
  });

  test('deep link to a specific journal date renders the page', async ({ page }) => {
    const targetDate = '2026-01-15';

    await page.goto(`${FRONTEND_URL}/journal/${targetDate}`);

    // The journal route should accept the date and render. The breadcrumb
    // is a stable element on every authenticated page; it formats the
    // date as e.g. "Thu, Jan 15, 2026", so we assert on the human-readable
    // fragments rather than the ISO string.
    const breadcrumb = page.getByTestId('breadcrumb');
    await expect(breadcrumb).toBeVisible({ timeout: 15_000 });
    await expect(breadcrumb).toContainText('Jan 15, 2026');

    // URL should still reflect the deep-linked date.
    expect(page.url()).toContain(`/journal/${targetDate}`);
  });
});

// ─── Test group: Auth ───────────────────────────────────────────────────────

test.describe('API Auth', () => {
  test('unauthenticated GET /api/v1/blocks returns 401', async ({ page }) => {
    // No Authorization header — request goes through page.request which
    // shares the page's context but does NOT inject any default auth.
    const response = await page.request.get(`${API_URL}/api/v1/blocks`);

    expect(response.status()).toBe(401);
  });

  test('authenticated GET /api/v1/blocks returns 200', async ({ page }) => {
    const response = await page.request.get(`${API_URL}/api/v1/blocks`, {
      headers: getAuthHeaders(),
    });

    expect(response.status()).toBe(200);
  });

  test('auth key persists across multiple navigations', async ({ page }) => {
    // Root redirects to /select-graph (no graph selected yet)
    await page.goto(`${FRONTEND_URL}/`);
    await page.waitForURL(/\/select-graph/, { timeout: 15_000 });

    // Navigate directly to /pages (deep link, bypasses graph selection)
    await page.goto(`${FRONTEND_URL}/pages`);
    await expect(page.getByTestId('app-shell')).toBeVisible({ timeout: 10_000 });

    // Navigate to /search
    await page.goto(`${FRONTEND_URL}/search`);
    await expect(page.getByTestId('app-shell')).toBeVisible({ timeout: 10_000 });

    // Back to the journal — a real authenticated call from the browser.
    // If the auth key were dropped, the journal would 401 and the page
    // wouldn't render the breadcrumb. We assert the breadcrumb as proof
    // that authenticated data was fetched.
    await page.goto(`${FRONTEND_URL}/journal/${todayLocalDate()}`);
    const breadcrumb = page.getByTestId('breadcrumb');
    await expect(breadcrumb).toBeVisible({ timeout: 15_000 });

    // Final check: a direct API call with the same header still works.
    const response = await page.request.get(`${API_URL}/api/v1/blocks`, {
      headers: getAuthHeaders(),
    });
    expect(response.status()).toBe(200);
  });
});

// ─── Test group: API Health ─────────────────────────────────────────────────

test.describe('Health Endpoint', () => {
  test('GET /health returns { status: "ok" }', async ({ page }) => {
    const response = await page.request.get(`${API_URL}/health`);

    expect(response.status()).toBe(200);

    const body = await response.json();
    expect(body).toEqual({ status: 'ok' });
  });

  test('health endpoint stays available during SPA navigation', async ({ page }) => {
    // Navigate to journal
    await page.goto(`${FRONTEND_URL}/journal/${todayLocalDate()}`);
    await expect(page.getByTestId('app-shell')).toBeVisible({ timeout: 15_000 });

    // Health still good
    const r1 = await page.request.get(`${API_URL}/health`);
    expect(r1.status()).toBe(200);
    expect(await r1.json()).toEqual({ status: 'ok' });

    // Navigate to pages
    await page.goto(`${FRONTEND_URL}/pages`);
    await expect(page.getByTestId('app-shell')).toBeVisible({ timeout: 10_000 });

    // Health still good
    const r2 = await page.request.get(`${API_URL}/health`);
    expect(r2.status()).toBe(200);
    expect(await r2.json()).toEqual({ status: 'ok' });
  });
});

// ─── Test group: Error handling ─────────────────────────────────────────────

test.describe('Error Handling', () => {
  test('non-existent block uuid returns 404', async ({ page }) => {
    // DELETE is the only verb on /api/v1/blocks/:id that actually
    // returns 404 for a missing uuid. GET is 405 (Method Not Allowed)
    // because the route only defines DELETE and PATCH handlers.
    const response = await page.request.delete(
      `${API_URL}/api/v1/blocks/00000000-0000-0000-0000-000000000000`,
      { headers: getAuthHeaders() }
    );

    expect(response.status()).toBe(404);
  });

  test('non-existent SPA route renders a not-found message', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/non-existent-page-xyz`);

    // Match the pattern used in error-handling.spec.ts: look for any
    // visible element whose text mentions "not found" / "404" /
    // "doesn't exist". We deliberately avoid CSS-class-only selectors.
    const notFound = page
      .getByText(/not found|404|doesn.t exist|page not found/i)
      .first();

    await expect(notFound).toBeVisible({ timeout: 10_000 });
  });
});
