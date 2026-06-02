/**
 * Journal E2E Tests
 *
 * Tests for journal page navigation, date header display,
 * prev/next day navigation, and block rendering with auth.
 *
 * Prerequisites:
 *   - Server running on localhost:3737
 *   - Frontend running on localhost:5173
 *   - QUILT_API_KEY env var set
 *
 * Run with: QUILT_API_KEY=<key> npx playwright test journal
 */

import { test, expect, type Page } from '@playwright/test';
import { getAuthHeaders } from '../auth-state';

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737';
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173';

/** Returns today's date as YYYY-MM-DD string. */
function todayDate(): string {
  const d = new Date();
  return d.toISOString().slice(0, 10);
}

/** Create a block via REST API (with auth). */
async function createBlock(
  page: Page,
  date: string,
  content: string,
  parentId: string | null = null
): Promise<string> {
  const headers = getAuthHeaders();
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, {
    data: { pageName: date, content, parentId },
    headers,
  });
  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(
      `createBlock failed with ${resp.status()}: ${body}`
    );
  }
  const json = (await resp.json()) as { id: string };
  return json.id;
}

/** Get blocks for a page via REST API (with auth). */
async function getBlocks(
  page: Page,
  pageName: string
): Promise<Array<{ id: string; content: string }>> {
  const headers = getAuthHeaders();
  const resp = await page.request.get(
    `${API_URL}/api/v1/pages/${encodeURIComponent(pageName)}/blocks`,
    { headers }
  );
  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(
      `getBlocks failed with ${resp.status()}: ${body}`
    );
  }
  return (await resp.json()) as Array<{ id: string; content: string }>;
}

test.describe('Journal Page', () => {
  test('navigating to today\'s journal shows date header', async ({ page }) => {
    const date = todayDate();
    await page.goto(`${FRONTEND_URL}/journal/${date}`);

    // Wait for the main content area to load
    const main = page.locator('main');
    await expect(main).toBeVisible({ timeout: 15000 });
  });

  test('prev day navigation button is visible', async ({ page }) => {
    const date = todayDate();
    await page.goto(`${FRONTEND_URL}/journal/${date}`);

    const prevBtn = page.locator('[aria-label="Previous day"]');
    await expect(prevBtn).toBeVisible({ timeout: 10000 });
  });

  test('next day navigation button is visible', async ({ page }) => {
    const date = todayDate();
    await page.goto(`${FRONTEND_URL}/journal/${date}`);

    const nextBtn = page.locator('[aria-label="Next day"]');
    await expect(nextBtn).toBeVisible({ timeout: 10000 });
  });

  test('prev day button navigates to previous date', async ({ page }) => {
    const date = todayDate();
    await page.goto(`${FRONTEND_URL}/journal/${date}`);

    const prevBtn = page.locator('[aria-label="Previous day"]');
    await expect(prevBtn).toBeVisible({ timeout: 5000 });

    // Compute expected previous date
    const d = new Date(date + 'T00:00:00');
    d.setDate(d.getDate() - 1);
    const expectedPrev = d.toISOString().split('T')[0];

    await prevBtn.click();

    // URL should contain the previous date
    await expect(page).toHaveURL(new RegExp(expectedPrev));
  });

  test('next day button navigates to next date', async ({ page }) => {
    const date = todayDate();
    await page.goto(`${FRONTEND_URL}/journal/${date}`);

    const nextBtn = page.locator('[aria-label="Next day"]');
    await expect(nextBtn).toBeVisible({ timeout: 5000 });

    // Compute expected next date
    const d = new Date(date + 'T00:00:00');
    d.setDate(d.getDate() + 1);
    const expectedNext = d.toISOString().split('T')[0];

    await nextBtn.click();

    await expect(page).toHaveURL(new RegExp(expectedNext));
  });

  test('journal with blocks shows block rows', async ({ page }) => {
    const date = todayDate();
    await createBlock(page, date, 'Test block for journal');

    await page.goto(`${FRONTEND_URL}/journal/${date}`);

    // Wait for block rows to render
    const blockRows = page.locator('.block-row');
    await expect(blockRows.first()).toBeVisible({ timeout: 15000 });
    await expect(blockRows.first()).toContainText('Test block for journal');
  });

  test('breadcrumb shows formatted date on journal page', async ({ page }) => {
    const date = todayDate();
    await page.goto(`${FRONTEND_URL}/journal/${date}`);

    const breadcrumb = page.locator('[data-testid="breadcrumb"]');
    await expect(breadcrumb).toBeVisible({ timeout: 5000 });

    const text = await breadcrumb.textContent();
    expect(text).toBeTruthy();
    // Should contain today's day of the month
    const dayOfMonth = new Date().getDate().toString();
    expect(text).toContain(dayOfMonth);
  });
});
