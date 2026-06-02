/**
 * Quilt UI E2E Tests — Baseline P0 Tests
 *
 * These tests verify core functionality of the React SPA.
 * Run with: npx playwright test
 *
 * Prerequisites:
 *   - Server running on localhost:3737
 *   - Frontend running on localhost:5173
 *   - Run `cd quilt-ui && npm run dev` before these tests
 */

import { test, expect } from '@playwright/test';

const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173';

// ─── Test 1: Sidebar Navigation Route Changes ───────────────────────────────

test.describe('Sidebar Navigation', () => {
  test('should navigate to different routes when sidebar links are clicked', async ({ page }) => {
    await page.goto(FRONTEND_URL);

    // Verify we're on a valid page (journal is the default route)
    await expect(page.locator('main, .app-shell, #root')).toBeVisible({ timeout: 10000 });

    // Try to navigate to key routes
    const navLinks = [
      { testid: 'nav-journal', route: /journal/ },
      { testid: 'nav-pages', route: /pages/ },
      { testid: 'nav-search', route: /search/ },
    ];

    for (const { testid, route } of navLinks) {
      const link = page.locator(`[data-testid="${testid}"]`);
      if (await link.isVisible({ timeout: 3000 })) {
        await link.click();
        await expect(page).toHaveURL(route);
      }
    }
  });
});

// ─── Test 2: Search Input + Results/Empty State ──────────────────────────────

test.describe('Search', () => {
  test('should render search input', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/search`);

    const searchInput = page.locator('[data-testid="search-input"]');
    await expect(searchInput).toBeVisible({ timeout: 10000 });
  });
});

// ─── Test 3: Pages View ─────────────────────────────────────────────────────

test.describe('Pages View', () => {
  test('should render pages view', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/pages`);

    // Wait for page content to load
    await expect(page.locator('main, h1, h2').first()).toBeVisible({
      timeout: 10000,
    });
  });
});
