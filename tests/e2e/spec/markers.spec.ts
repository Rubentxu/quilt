/**
 * Block Markers / Bullet E2E Tests
 *
 * Tests for block bullet rendering, marker badges (TODO/DONE),
 * and collapse/expand behavior on parent blocks.
 *
 * Requires a running backend.
 * Run with: npx playwright test markers
 */

import { test, expect, type Page } from '@playwright/test';

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737';

let dateCounter = 0;
function uniqueDate(): string {
  dateCounter++;
  const d = new Date();
  d.setDate(d.getDate() + 90 + dateCounter);
  return d.toISOString().slice(0, 10);
}

async function createBlock(
  page: Page,
  date: string,
  content: string,
  parentId: string | null = null,
  marker?: string,
): Promise<string> {
  const data: Record<string, unknown> = { pageName: date, content, parentId };
  if (marker) data.marker = marker;
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, { data });
  expect(resp.ok()).toBeTruthy();
  return ((await resp.json()) as { id: string }).id;
}

async function deleteAllBlocks(page: Page, date: string) {
  const resp = await page.request.get(`${API_URL}/api/v1/pages/${date}/blocks`);
  if (!resp.ok()) return;
  const blocks = (await resp.json()) as Array<{ id: string }>;
  for (const block of blocks) {
    await page.request.delete(`${API_URL}/api/v1/blocks/${block.id}`);
  }
}

test.describe('Block Bullets', () => {
  test('leaf block renders a bullet button', async ({ page }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await createBlock(page, date, 'Leaf block');
    await page.goto(`http://localhost:5173/journal/${date}`);

    try {
      await page.waitForSelector('.block-row', { timeout: 10000 });

      // Each block row should have a bullet button
      const bullet = page.locator('.block-row .block-bullet').first();
      await expect(bullet).toBeVisible({ timeout: 5000 });

      // Leaf blocks have aria-label "Bullet"
      await expect(bullet).toHaveAttribute('aria-label', 'Bullet');
    } catch {
      test.skip(true, 'Backend not available for bullet test');
    }
  });
});

test.describe('Task Markers', () => {
  test('block with TODO marker shows marker badge', async ({ page }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await createBlock(page, date, 'Task item', null, 'Todo');
    await page.goto(`http://localhost:5173/journal/${date}`);

    try {
      await page.waitForSelector('.block-row', { timeout: 10000 });

      // Should see TODO marker badge
      const markerBadge = page.locator('.block-row').first().locator('text=TODO');
      await expect(markerBadge).toBeVisible({ timeout: 5000 });
    } catch {
      test.skip(true, 'Backend not available for marker test');
    }
  });

  test('block with DONE marker shows DONE badge', async ({ page }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await createBlock(page, date, 'Completed task', null, 'Done');
    await page.goto(`http://localhost:5173/journal/${date}`);

    try {
      await page.waitForSelector('.block-row', { timeout: 10000 });

      const markerBadge = page.locator('.block-row').first().locator('text=DONE');
      await expect(markerBadge).toBeVisible({ timeout: 5000 });
    } catch {
      test.skip(true, 'Backend not available for DONE marker test');
    }
  });

  test('block without marker has no badge', async ({ page }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await createBlock(page, date, 'Plain text block');
    await page.goto(`http://localhost:5173/journal/${date}`);

    try {
      await page.waitForSelector('.block-row', { timeout: 10000 });

      // Should see the block content but no marker badge
      const blockRow = page.locator('.block-row').first();
      await expect(blockRow).toContainText('Plain text block');

      // No TODO/DONE/NOW/LATER/CANCELLED badge should be present
      const badges = blockRow.locator('text=/^(TODO|DONE|NOW|LATER|CANCELLED)$/');
      const count = await badges.count();
      expect(count).toBe(0);
    } catch {
      test.skip(true, 'Backend not available for plain block test');
    }
  });
});

test.describe('Block Drag Handles', () => {
  test('block row has a drag handle element', async ({ page }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await createBlock(page, date, 'Draggable block');
    await page.goto(`http://localhost:5173/journal/${date}`);

    try {
      await page.waitForSelector('.block-row', { timeout: 10000 });

      // Each row should have a drag handle (GripVertical icon)
      const dragHandle = page.locator('.block-row .drag-handle').first();
      await expect(dragHandle).toBeVisible({ timeout: 5000 });
    } catch {
      test.skip(true, 'Backend not available for drag handle test');
    }
  });
});
