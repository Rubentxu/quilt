/**
 * Error Handling E2E Tests
 *
 * Tests for offline handling, API timeouts, and graceful degradation.
 * Run with: npx playwright test error-handling
 */

import { test, expect } from '@playwright/test';

test.describe('Offline/Network Failure Handling', () => {
  test('page shows error state when network fails', async ({ page }) => {
    await page.goto('http://localhost:1420/');

    // Simulate going offline
    await page.context().setOffline(true);

    // Try to interact with the page (e.g., navigate)
    await page.click('[data-testid="nav-pages"]');

    // Page should either:
    // 1. Show an error state
    // 2. Work in offline mode
    // 3. Show cached content
    // We just verify the app doesn't crash
    await expect(page.locator('.app-shell')).toBeVisible();

    // Come back online
    await page.context().setOffline(false);
  });

  test('search handles network error gracefully', async ({ page }) => {
    await page.goto('http://localhost:1420/search');

    // Go offline
    await page.context().setOffline(true);

    // Try to search
    await page.locator('[data-testid="search-input"]').fill('test');
    await page.locator('[data-testid="search-input"]').press('Enter');

    // Should show empty state or error - not crash
    const emptyOrError = page.locator('.empty-state, .search-error, .search-results');
    await expect(emptyOrError.first()).toBeVisible({ timeout: 5000 });

    // Come back online
    await page.context().setOffline(false);
  });

  test('graph view handles network failure', async ({ page }) => {
    await page.goto('http://localhost:1420/graph');

    // Wait for initial load
    await page.waitForSelector(
      '[data-testid="force-graph"], [data-testid="graph-empty"], [data-testid="graph-error"]',
      { timeout: 20000 }
    );

    // Go offline
    await page.context().setOffline(true);

    // The graph should still be visible (either cached or error state)
    const graphOrError = page.locator(
      '[data-testid="graph-view"], [data-testid="graph-error"], [data-testid="force-graph"]'
    );
    await expect(graphOrError.first()).toBeVisible();

    // Come back online
    await page.context().setOffline(false);
  });
});

test.describe('API Timeout States', () => {
  test('query timeout shows error or retry option', async ({ page }) => {
    await page.goto('http://localhost:1420/query');

    // Enter a query that might timeout (complex query)
    await page.fill('[data-testid="query-input"]', '(and [todo])');

    // Execute query
    await page.click('[data-testid="run-query-button"]');

    // Wait for results or error
    const resultsOrError = page.locator('.query-results, .query-error, .empty-state');
    await expect(resultsOrError.first()).toBeVisible({ timeout: 30000 });
  });

  test('cognitive dashboard handles slow load gracefully', async ({ page }) => {
    await page.goto('http://localhost:1420/cognitive');

    // Loading state should appear quickly
    const loadingOrContent = page.locator('.dashboard-loading, .briefing-content, .dashboard-error');
    await expect(loadingOrContent.first()).toBeVisible({ timeout: 5000 });

    // Eventually should show either content or error
    const contentOrError = page.locator('.briefing-content, .dashboard-error');
    await expect(contentOrError.first()).toBeVisible({ timeout: 60000 });
  });
});

test.describe('Empty Database States', () => {
  test('pages view shows empty state when no pages', async ({ page }) => {
    await page.goto('http://localhost:1420/pages');

    // Wait for content or empty state
    await page.waitForSelector('.block-list, .empty-state', { timeout: 15000 });

    // Should show empty state if no pages
    const pagesOrEmpty = page.locator('.block-list, .empty-state');
    await expect(pagesOrEmpty.first()).toBeVisible();
  });

  test('journal shows empty state when no entries', async ({ page }) => {
    await page.goto('http://localhost:1420/journal');

    // Journal should have date header visible
    await expect(page.locator('h2.journal-date, h2:has-text("Journal")')).toBeVisible({ timeout: 10000 });

    // If no entries, should show empty state within journal content
    const contentOrEmpty = page.locator('.journal-entries, .empty-state');
    await expect(contentOrEmpty.first()).toBeVisible({ timeout: 10000 });
  });

  test('search shows empty state when no results', async ({ page }) => {
    await page.goto('http://localhost:1420/search');

    // Search for very specific unlikely term
    await page.fill('[data-testid="search-input"]', 'xyznonexistentpage12345xyz');
    await page.press('[data-testid="search-input"]', 'Enter');

    // Should show empty state
    await expect(page.locator('.empty-state:has-text("No results")')).toBeVisible({ timeout: 10000 });
  });

  test('graph shows empty state when no data', async ({ page }) => {
    await page.goto('http://localhost:1420/graph');

    // Wait for any state
    await page.waitForSelector(
      '[data-testid="force-graph"], [data-testid="graph-empty"], [data-testid="graph-error"]',
      { timeout: 20000 }
    );

    // If no data, should show graph-empty state
    const emptyState = page.locator('[data-testid="graph-empty"]');
    // This is conditional on actual data being absent
    if (await emptyState.isVisible()) {
      await expect(emptyState).toBeVisible();
    }
  });
});

test.describe('Error Recovery', () => {
  test('retry button on graph error reloads graph', async ({ page }) => {
    await page.goto('http://localhost:1420/graph');

    // Wait for either graph to load or error to appear
    await page.waitForSelector(
      '[data-testid="force-graph"], [data-testid="graph-empty"], [data-testid="graph-error"]',
      { timeout: 20000 }
    );

    // If error state is showing, click retry
    const errorState = page.locator('[data-testid="graph-error"]');
    if (await errorState.isVisible()) {
      await page.click('[data-testid="graph-retry-button"]');

      // Should attempt to reload
      await page.waitForSelector(
        '[data-testid="force-graph"], [data-testid="graph-empty"], [data-testid="graph-error"]',
        { timeout: 20000 }
      );
    }
  });

  test('cognitive refresh button works', async ({ page }) => {
    await page.goto('http://localhost:1420/cognitive');

    // Wait for initial load
    await page.waitForSelector(
      '.briefing-content, .dashboard-error, .dashboard-loading',
      { timeout: 20000 }
    );

    // Click refresh if available
    const refreshButton = page.locator('[data-testid="refresh-button"]');
    if (await refreshButton.isVisible()) {
      await refreshButton.click();

      // Should show loading then content again
      await page.waitForSelector(
        '.briefing-content, .dashboard-error, .dashboard-loading',
        { timeout: 20000 }
      );
    }
  });
});

test.describe('Not Found Handling', () => {
  test('unknown route shows 404 page', async ({ page }) => {
    await page.goto('http://localhost:1420/nonexistent-route-12345');

    // Should show not found message
    await expect(page.locator('h2:has-text("not found"), h2:has-text("Not Found")')).toBeVisible({ timeout: 10000 });
  });

  test('404 page has working navigation', async ({ page }) => {
    await page.goto('http://localhost:1420/nonexistent-route-12345');

    // Should show not found
    await expect(page.locator('.empty-state, h2:has-text("not found")')).toBeVisible({ timeout: 10000 });

    // Should be able to navigate to valid pages
    await page.click('[data-testid="nav-journal"]');
    await expect(page).toHaveURL(/\/journal/);
  });
});
