/**
 * Error Handling E2E Tests
 *
 * Tests for offline handling, 404 pages, and graceful degradation.
 * These tests verify the frontend handles error states without crashing.
 *
 * Prerequisites:
 *   - Frontend running on localhost:5173
 *
 * Run with: npx playwright test error-handling
 */

import { test, expect } from '@playwright/test';

const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173';

test.describe('Not Found Handling', () => {
  test('unknown route shows not-found page', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/nonexistent-route-12345`);

    // Should show a not-found message
    await expect(
      page.locator(
        '.empty-state, h1, h2'
      ).filter({ hasText: /not found|404|doesn.t exist/i })
    ).toBeVisible({ timeout: 10000 });
  });

  test('not-found page has working navigation', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/nonexistent-route-12345`);

    // Click journal nav to return to a valid page
    const journalNav = page.locator('[data-testid="nav-journal"]');
    if (await journalNav.isVisible({ timeout: 5000 })) {
      await journalNav.click();
      await expect(page).toHaveURL(/\/journal/);
    }
  });
});

test.describe('Offline/Network Failure Handling', () => {
  test('app shell remains visible when offline', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/`);

    // Simulate going offline
    await page.context().setOffline(true);

    // The app shell should still be visible (cached SPA)
    await expect(page.locator('main, .app-shell, #root')).toBeVisible({
      timeout: 5000,
    });

    // Come back online
    await page.context().setOffline(false);
  });

  test('search input is visible on search page', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/search`);

    const searchInput = page.locator('[data-testid="search-input"]');
    await expect(searchInput).toBeVisible({ timeout: 10000 });

    // Offline should not crash the search page
    await page.context().setOffline(true);

    // UI should still be rendered
    await expect(page.locator('main, .app-shell, #root')).toBeVisible();

    await page.context().setOffline(false);
  });
});

test.describe('Empty State Handling', () => {
  test('journal page renders with or without blocks', async ({ page }) => {
    const d = new Date();
    const date = d.toISOString().slice(0, 10);
    await page.goto(`${FRONTEND_URL}/journal/${date}`);

    // Journal should have a main content area
    await expect(page.locator('main, .app-shell')).toBeVisible({
      timeout: 10000,
    });
  });

  test('pages view renders with header', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/pages`);

    // Should show some page-level UI
    await expect(page.locator('main, h1, h2').first()).toBeVisible({
      timeout: 10000,
    });
  });
});
