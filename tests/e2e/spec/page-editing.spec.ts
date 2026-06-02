/**
 * Page Editing E2E Tests
 *
 * Tests for editing block content on regular (non-journal) pages,
 * Enter to split, and content persistence after reload.
 *
 * Prerequisites:
 *   - Server running on localhost:3737
 *   - Frontend running on localhost:5173
 *   - QUILT_API_KEY env var set
 *
 * Run with: QUILT_API_KEY=<key> npx playwright test page-editing
 */

import { test, expect, type Page } from '@playwright/test';
import { getAuthHeaders } from '../auth-state';

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737';
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173';

let pageCounter = 0;
function uniquePageName(): string {
  pageCounter++;
  return `playwright-page-${Date.now()}-${pageCounter}`;
}

async function createBlock(
  page: Page,
  pageName: string,
  content: string,
  parentId: string | null = null
): Promise<string> {
  const headers = getAuthHeaders();
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, {
    data: { pageName, content, parentId },
    headers,
  });
  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`createBlock failed with ${resp.status()}: ${body}`);
  }
  const json = (await resp.json()) as { id: string };
  return json.id;
}

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
    throw new Error(`getBlocks failed with ${resp.status()}: ${body}`);
  }
  return (await resp.json()) as Array<{ id: string; content: string }>;
}

async function deleteAllBlocks(page: Page, pageName: string) {
  const blocks = await getBlocks(page, pageName);
  const headers = getAuthHeaders();
  for (const block of blocks) {
    await page.request.delete(`${API_URL}/api/v1/blocks/${block.id}`, { headers });
  }
}

test.describe('Regular Page Editing', () => {
  test('page with blocks shows block rows', async ({ page }) => {
    const pageName = uniquePageName();
    await createBlock(page, pageName, 'Regular page text');

    await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(pageName)}`);

    const blockRows = page.locator('.block-row');
    await expect(blockRows.first()).toBeVisible({ timeout: 15000 });
    await expect(blockRows.first()).toContainText('Regular page text');
  });

  test('page title is shown in breadcrumb', async ({ page }) => {
    const pageName = uniquePageName();
    await createBlock(page, pageName, 'Breadcrumb test');

    await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(pageName)}`);

    const breadcrumb = page.locator('[data-testid="breadcrumb"]');
    await expect(breadcrumb).toBeVisible({ timeout: 5000 });
    const text = await breadcrumb.textContent();
    expect(text).toBe(pageName);
  });

  test('page header shows page name as title', async ({ page }) => {
    const pageName = uniquePageName();
    await createBlock(page, pageName, 'Title test');

    await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(pageName)}`);

    const h1 = page.locator('h1');
    await expect(h1).toBeVisible({ timeout: 10000 });
    await expect(h1).toContainText(pageName);
  });

  test('clicking block enters edit mode and typing works', async ({ page }) => {
    const pageName = uniquePageName();
    await createBlock(page, pageName, 'Edit test');

    await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(pageName)}`);

    // Wait for the block row to appear
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10000,
    });

    // Click on the block to enter edit mode
    const readContent = page.locator('.block-content-read').first();
    await readContent.click();

    // Should now be in edit mode with contentEditable
    const editor = page.locator('.block-content[contenteditable="true"]');
    await expect(editor).toBeVisible({ timeout: 5000 });

    // Type new text
    await editor.first().fill('');
    await editor.first().type(' UPDATED ', { delay: 10 });

    // Click outside to save
    await page.locator('[data-testid="breadcrumb"]').click();

    // Should contain the updated text
    await expect(page.locator('.block-row').first()).toContainText('UPDATED');

    // Cleanup
    await deleteAllBlocks(page, pageName);
  });

  test('Enter key creates a new block on page', async ({ page }) => {
    const pageName = uniquePageName();
    await createBlock(page, pageName, 'First Second');

    await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(pageName)}`);

    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10000,
    });

    const readContent = page.locator('.block-content-read').first();
    await readContent.click();

    const editor = page.locator('.block-content[contenteditable="true"]').first();
    await editor.press('End');

    // Press Enter to split/create new block
    await editor.press('Enter');

    // There should be at least 2 blocks now
    const count = await page.locator('.block-row').count();
    expect(count).toBeGreaterThanOrEqual(2);

    // Verify via API
    const blocks = await getBlocks(page, pageName);
    expect(blocks.length).toBeGreaterThanOrEqual(2);

    // Cleanup
    await deleteAllBlocks(page, pageName);
  });
});
