/**
 * Quilt Graph View E2E Tests
 *
 * Tests canvas-based force-directed graph visualization.
 * Run with: npx playwright test graph-view
 *
 * Prerequisites:
 * - Dev server running on http://localhost:1420
 * - Run `cd crates/quilt-ui && trunk serve --port 1420` before these tests
 */

import { test, expect } from '@playwright/test';

// ─── Test 1: Sidebar Navigation to Graph View ─────────────────────────────

test.describe('Graph View Navigation', () => {
  test('should navigate to graph view via sidebar link', async ({ page }) => {
    await page.goto('/');

    // Click Graph navigation link
    await page.click('[data-testid="nav-graph"]');
    await expect(page).toHaveURL(/\/graph/);

    // Verify page header is visible
    await expect(page.locator('[data-testid="graph-title"]')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('[data-testid="graph-title"]')).toHaveText('Vista Grafo');
  });

  test('should display graph view container', async ({ page }) => {
    await page.goto('/graph');

    // Verify main graph container exists
    await expect(page.locator('[data-testid="graph-view"]')).toBeVisible({ timeout: 10000 });
  });
});

// ─── Test 2: Graph Loading and Content States ─────────────────────────────

test.describe('Graph View States', () => {
  test('should show loading state then content or empty state', async ({ page }) => {
    await page.goto('/graph');

    // Wait for loading to complete (either content, empty, or error state)
    const contentOrEmpty = page.locator(
      '[data-testid="force-graph"], [data-testid="graph-empty"], [data-testid="graph-error"]'
    );
    await expect(contentOrEmpty.first()).toBeVisible({ timeout: 20000 });
  });

  test('should display subtitle text', async ({ page }) => {
    await page.goto('/graph');

    // Verify subtitle
    await expect(page.locator('.page-subtitle:has-text("Navega tu conocimiento como una red")')).toBeVisible({ timeout: 10000 });
  });
});

// ─── Test 3: Graph Canvas and Controls ────────────────────────────────────

test.describe('Graph View Canvas', () => {
  test('should display canvas when graph has data', async ({ page }) => {
    await page.goto('/graph');

    // Wait for canvas to appear (or empty/error state)
    await page.waitForSelector(
      '[data-testid="force-graph"], [data-testid="graph-empty"], [data-testid="graph-error"]',
      { timeout: 20000 }
    );

    // If force-graph loaded (not empty/error), verify canvas and controls
    const forceGraph = page.locator('[data-testid="force-graph"]');
    if (await forceGraph.isVisible()) {
      // Canvas should be visible
      await expect(page.locator('[data-testid="graph-canvas"]')).toBeVisible();

      // Legend should be visible
      await expect(page.locator('[data-testid="graph-legend"]')).toBeVisible();

      // Controls should be visible
      await expect(page.locator('[data-testid="graph-controls"]')).toBeVisible();

      // Filter buttons should exist
      await expect(page.locator('[data-testid="graph-filter-pages"]')).toBeVisible();
      await expect(page.locator('[data-testid="graph-filter-journals"]')).toBeVisible();

      // Zoom controls should exist
      await expect(page.locator('[data-testid="zoom-in"]')).toBeVisible();
      await expect(page.locator('[data-testid="zoom-reset"]')).toBeVisible();
      await expect(page.locator('[data-testid="zoom-out"]')).toBeVisible();
    }
  });

  test('should display graph controls for filtering', async ({ page }) => {
    await page.goto('/graph');

    // Wait for force-graph
    await page.waitForSelector('[data-testid="force-graph"]', { timeout: 20000 }).catch(() => {});

    const forceGraph = page.locator('[data-testid="force-graph"]');
    if (await forceGraph.isVisible()) {
      // Filter pages button
      const pagesBtn = page.locator('[data-testid="graph-filter-pages"]');
      await expect(pagesBtn).toBeVisible();
      // Should be active by default
      await expect(pagesBtn).toHaveClass(/active/);

      // Filter journals button
      const journalsBtn = page.locator('[data-testid="graph-filter-journals"]');
      await expect(journalsBtn).toBeVisible();
      await expect(journalsBtn).toHaveClass(/active/);
    }
  });
});

// ─── Test 4: Graph Error Handling ─────────────────────────────────────────

test.describe('Graph View Error Handling', () => {
  test('should handle errors gracefully with retry button', async ({ page }) => {
    await page.goto('/graph');

    // Wait for any state to settle
    await page.waitForSelector(
      '[data-testid="force-graph"], [data-testid="graph-empty"], [data-testid="graph-error"]',
      { timeout: 20000 }
    );

    // If error state is showing, verify retry button exists
    const errorState = page.locator('[data-testid="graph-error"]');
    if (await errorState.isVisible()) {
      await expect(page.locator('[data-testid="graph-retry-button"]')).toBeVisible();
      await expect(page.locator('[data-testid="graph-error-message"]')).toBeVisible();
    }
  });
});

// ─── Test 5: Graph Navigation Roundtrip ───────────────────────────────────

test.describe('Graph View Navigation Roundtrip', () => {
  test('should navigate from graph to pages and back', async ({ page }) => {
    await page.goto('/graph');
    await expect(page.locator('[data-testid="graph-view"]')).toBeVisible({ timeout: 10000 });

    // Navigate to Pages
    await page.click('[data-testid="nav-pages"]');
    await expect(page).toHaveURL(/\/pages/);
    await expect(page.locator('h2:has-text("Pages")')).toBeVisible();

    // Navigate back to Graph
    await page.click('[data-testid="nav-graph"]');
    await expect(page).toHaveURL(/\/graph/);
    await expect(page.locator('[data-testid="graph-title"]')).toBeVisible();
  });
});
