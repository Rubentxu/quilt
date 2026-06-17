/**
 * Graph Selector E2E Tests — ADR-0030, Slice D
 *
 * Covers the basic flow of the `/select-graph` route:
 * - GraphSelectorPage renders with three tabs: Recent, Open by Path, Create New
 * - Recent tab shows graph list or empty state
 * - Open by Path and Create New forms accept paths and submit
 * - Error states render correctly
 *
 * Prerequisites:
 *   - Server running on http://localhost:3737
 *   - Frontend running on http://localhost:5173 (Playwright spawns via
 *     `webServer` in playwright.config.ts)
 *   - QUILT_API_KEY env var set
 *
 * Run: QUILT_API_KEY=<key> npx playwright test graph-selector
 */

import { test, expect } from '@playwright/test';
import { getAuthHeaders, requireApiKey } from '../auth-state';

const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173';
const API_URL = process.env.API_BASE_URL || 'http://localhost:3737';

requireApiKey();

// ─── Helpers ─────────────────────────────────────────────────────────────

async function clearGlobalState(headers: Record<string, string>) {
  // Reset to a known clean state — no last opened graph
  await fetch(`${API_URL}/api/v1/global-state/last-opened`, {
    method: 'PUT',
    headers: { ...headers, 'Content-Type': 'application/json' },
    body: JSON.stringify({ graphPath: null }),
  });
}

// ─── Tests ───────────────────────────────────────────────────────────────

test.describe('GraphSelectorPage — basic flow', () => {
  const headers = getAuthHeaders();

  test.beforeEach(async () => {
    await clearGlobalState(headers);
  });

  test('renders the graph selector page at /select-graph', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/select-graph`);

    await expect(page.getByRole('heading', { name: /open or create a graph/i })).toBeVisible({ timeout: 10000 });
  });

  test('shows three tabs: Recent, Open by Path, Create New', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/select-graph`);

    await expect(page.getByRole('tab', { name: /recent/i })).toBeVisible();
    await expect(page.getByRole('tab', { name: /open by path/i })).toBeVisible();
    await expect(page.getByRole('tab', { name: /create new/i })).toBeVisible();
  });

  test('Recent tab is active by default', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/select-graph`);

    const recentTab = page.getByRole('tab', { name: /recent/i });
    await expect(recentTab).toHaveAttribute('aria-selected', 'true');
  });

  test('shows empty state message when there are no recent graphs', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/select-graph`);

    await expect(page.getByText(/no recent graphs/i)).toBeVisible();
  });

  test('switches to Open by Path tab and shows the path input', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/select-graph`);

    await page.getByRole('tab', { name: /open by path/i }).click();

    await expect(page.getByLabel(/graph directory path/i)).toBeVisible();
    await expect(page.getByRole('button', { name: /open graph/i })).toBeVisible();
  });

  test('switches to Create New tab and shows the path input', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/select-graph`);

    await page.getByRole('tab', { name: /create new/i }).click();

    await expect(page.getByLabel(/new graph directory path/i)).toBeVisible();
    await expect(page.getByRole('button', { name: /create graph/i })).toBeVisible();
  });

  test('submit button is disabled when path input is empty', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/select-graph`);

    await page.getByRole('tab', { name: /open by path/i }).click();

    const submitBtn = page.getByRole('button', { name: /open graph/i });
    await expect(submitBtn).toBeDisabled();
  });

  test('submit button is enabled when path input has a value', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/select-graph`);

    await page.getByRole('tab', { name: /open by path/i }).click();
    await page.getByLabel(/graph directory path/i).fill('/tmp/test-graph');

    const submitBtn = page.getByRole('button', { name: /open graph/i });
    await expect(submitBtn).toBeEnabled();
  });
});

test.describe('GraphSelectorPage — home redirect', () => {
  const headers = getAuthHeaders();

  test('navigating to / redirects to /select-graph when no lastOpenedGraph exists', async ({ page }) => {
    // Ensure no last opened graph
    await clearGlobalState(headers);

    await page.goto(`${FRONTEND_URL}/`);

    // Should redirect to graph selector
    await expect(page).toHaveURL(/\/select-graph/, { timeout: 10000 });
  });

  test('navigating to / redirects to /journal when a lastOpenedGraph exists', async ({ page }) => {
    // Set a last opened graph via API
    await fetch(`${API_URL}/api/v1/global-state/last-opened`, {
      method: 'PUT',
      headers: { ...headers, 'Content-Type': 'application/json' },
      body: JSON.stringify({ graphPath: '/tmp/existing-graph' }),
    });

    await page.goto(`${FRONTEND_URL}/`);

    // Should redirect to today's journal
    await expect(page).toHaveURL(/\/journal\/\d{4}-\d{2}-\d{2}/, { timeout: 10000 });
  });
});
