/**
 * Quilt Graph View E2E Tests
 *
 * Tests graph visualization page using stable data-testid selectors.
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
      '[data-testid="graph-content"], [data-testid="graph-empty"], [data-testid="graph-error"]'
    );
    await expect(contentOrEmpty.first()).toBeVisible({ timeout: 20000 });
  });

  test('should display subtitle text', async ({ page }) => {
    await page.goto('/graph');

    // Verify subtitle
    await expect(page.locator('.page-subtitle:has-text("Navega tu conocimiento como una red")')).toBeVisible({ timeout: 10000 });
  });
});

// ─── Test 3: Graph Content (when data exists) ─────────────────────────────

test.describe('Graph View Content', () => {
  test('should display stats when graph has data', async ({ page }) => {
    await page.goto('/graph');

    // Wait for loading to complete
    await page.waitForSelector(
      '[data-testid="graph-content"], [data-testid="graph-empty"], [data-testid="graph-error"]',
      { timeout: 20000 }
    );

    // If content loaded (not empty/error), verify stats
    const content = page.locator('[data-testid="graph-content"]');
    if (await content.isVisible()) {
      // Stats should be visible
      await expect(page.locator('[data-testid="graph-stats"]')).toBeVisible();
      await expect(page.locator('[data-testid="graph-stat-nodes"]')).toBeVisible();
      await expect(page.locator('[data-testid="graph-stat-edges"]')).toBeVisible();
      await expect(page.locator('[data-testid="graph-stat-journals"]')).toBeVisible();

      // Legend should be visible
      await expect(page.locator('[data-testid="graph-legend"]')).toBeVisible();

      // Visualization area should be visible
      await expect(page.locator('[data-testid="graph-visualization"]')).toBeVisible();
      await expect(page.locator('[data-testid="graph-nodes-grid"]')).toBeVisible();
    }
  });

  test('should display node cards when graph has data', async ({ page }) => {
    await page.goto('/graph');

    // Wait for content to load
    await page.waitForSelector(
      '[data-testid="graph-content"], [data-testid="graph-empty"]',
      { timeout: 20000 }
    );

    const content = page.locator('[data-testid="graph-content"]');
    if (await content.isVisible()) {
      // Should have at least one node card
      const nodes = page.locator('[data-testid^="graph-node-"]');
      const nodeCount = await nodes.count();
      expect(nodeCount).toBeGreaterThan(0);

      // Each node should have a name and type label
      const firstNode = nodes.first();
      await expect(firstNode.locator('.node-name')).toBeVisible();
      await expect(firstNode.locator('.node-type')).toBeVisible();
    }
  });
});

// ─── Test 4: Graph Error Handling ─────────────────────────────────────────

test.describe('Graph View Error Handling', () => {
  test('should handle errors gracefully with retry button', async ({ page }) => {
    // This test verifies the error state UI exists in the DOM structure
    // Actual error triggering requires backend manipulation
    await page.goto('/graph');

    // Wait for any state to settle
    await page.waitForSelector(
      '[data-testid="graph-content"], [data-testid="graph-empty"], [data-testid="graph-error"]',
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
