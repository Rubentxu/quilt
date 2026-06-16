/**
 * WASM Projection E2E Smoke (ADR-0028)
 *
 * Verifies that:
 * 1. Opening a page with task blocks triggers projection resolutions.
 * 2. The `window.__quiltProjectionMetrics` global is updated after
 *    resolutions complete.
 * 3. Either the WASM path OR the HTTP path serves the requests
 *    (depends on whether `wasm-pack` has been run yet in the test
 *    env). The test asserts the total count is > 0 (the system is
 *    working) without asserting a specific source.
 * 4. Reloading the page resets the metrics counters to zero.
 *
 * Requires a running backend (just dev — `just dev`). The tests
 * use the same auth pattern as the other e2e specs.
 */

import { test, expect, type Page } from '@playwright/test';

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737';

let dateCounter = 0;
function uniqueDate(): string {
  dateCounter++;
  const d = new Date();
  d.setDate(d.getDate() + 365 + dateCounter);
  return d.toISOString().slice(0, 10);
}

interface ProjectionMetrics {
  wasmCount: number;
  httpCount: number;
  httpErrorCount: number;
  wasmRatio: number;
}

async function readMetrics(page: Page): Promise<ProjectionMetrics | null> {
  return page.evaluate(
    () => (window as unknown as { __quiltProjectionMetrics?: ProjectionMetrics }).__quiltProjectionMetrics ?? null,
  );
}

async function createTaskBlock(page: Page, date: string, content: string): Promise<string> {
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, {
    data: { pageName: date, content, parentId: null },
  });
  expect(resp.ok()).toBeTruthy();
  return ((await resp.json()) as { id: string }).id;
}

async function setBlockTypeStatus(
  page: Page,
  blockId: string,
  type: string,
  status: string,
): Promise<void> {
  const resp = await page.request.put(`${API_URL}/api/v1/blocks/${blockId}/properties`, {
    data: { key: 'type', value: type },
  });
  expect(resp.ok()).toBeTruthy();
  const resp2 = await page.request.put(`${API_URL}/api/v1/blocks/${blockId}/properties`, {
    data: { key: 'status', value: status },
  });
  expect(resp2.ok()).toBeTruthy();
}

async function cleanupPage(page: Page, date: string): Promise<void> {
  // Delete all blocks on the page (best-effort; ignore errors).
  const resp = await page.request.get(`${API_URL}/api/v1/pages/${date}/blocks`);
  if (!resp.ok()) return;
  const blocks = (await resp.json()) as Array<{ id: string }>;
  for (const block of blocks) {
    await page.request.delete(`${API_URL}/api/v1/blocks/${block.id}`);
  }
}

test.describe('WASM projection metrics (ADR-0028)', () => {
  test('window.__quiltProjectionMetrics is defined on page load', async ({ page }) => {
    await page.goto('/');
    const metrics = await readMetrics(page);
    expect(metrics).not.toBeNull();
    // Initial state: zero counters
    expect(metrics!.wasmCount).toBe(0);
    expect(metrics!.httpCount).toBe(0);
    expect(metrics!.httpErrorCount).toBe(0);
    expect(metrics!.wasmRatio).toBe(0);
  });

  test('opening a page with task blocks increments metrics', async ({ page }) => {
    const date = uniqueDate();
    const blockId = await createTaskBlock(page, date, 'Buy milk');
    await setBlockTypeStatus(page, blockId, 'task', 'done');

    try {
      await page.goto(`/page/${date}`);
      // Wait for at least one projection resolution.
      await expect(async () => {
        const metrics = await readMetrics(page);
        const total = (metrics?.wasmCount ?? 0) + (metrics?.httpCount ?? 0);
        expect(total).toBeGreaterThan(0);
      }).toPass({ timeout: 10_000 });

      const metrics = await readMetrics(page);
      // At least one successful resolution (either WASM or HTTP).
      const total = metrics!.wasmCount + metrics!.httpCount;
      expect(total).toBeGreaterThan(0);
      // wasmRatio is consistent with the counts.
      const expectedRatio = metrics!.wasmCount / total;
      expect(metrics!.wasmRatio).toBeCloseTo(expectedRatio, 5);
    } finally {
      await cleanupPage(page, date);
    }
  });

  test('metrics reset to zero on page reload', async ({ page }) => {
    const date = uniqueDate();
    await createTaskBlock(page, date, 'Reload me');
    try {
      await page.goto(`/page/${date}`);
      // Wait for at least one resolution.
      await expect(async () => {
        const metrics = await readMetrics(page);
        const total = (metrics?.wasmCount ?? 0) + (metrics?.httpCount ?? 0);
        expect(total).toBeGreaterThan(0);
      }).toPass({ timeout: 10_000 });

      // Reload and assert counters reset.
      await page.reload();
      const metrics = await readMetrics(page);
      expect(metrics!.wasmCount).toBe(0);
      expect(metrics!.httpCount).toBe(0);
      expect(metrics!.httpErrorCount).toBe(0);
    } finally {
      await cleanupPage(page, date);
    }
  });

  test('debug panel shows live metrics when VITE_DEBUG_PANEL=true', async ({ page }) => {
    // Skip this test if the debug panel is not enabled in this env.
    test.skip(
      !process.env.VITE_DEBUG_PANEL,
      'VITE_DEBUG_PANEL is not set; debug panel is hidden',
    );

    const date = uniqueDate();
    await createTaskBlock(page, date, 'Debug me');
    try {
      await page.goto(`/page/${date}`);
      await expect(async () => {
        const metrics = await readMetrics(page);
        const total = (metrics?.wasmCount ?? 0) + (metrics?.httpCount ?? 0);
        expect(total).toBeGreaterThan(0);
      }).toPass({ timeout: 10_000 });

      // The debug panel renders the metrics in a known format. We
      // look for the substring "Projection:" which is unique to the
      // panel.
      const panelText = await page.locator('body').innerText();
      expect(panelText).toContain('Projection:');
    } finally {
      await cleanupPage(page, date);
    }
  });
});
