/**
 * Visual regression tests.
 *
 * Captures screenshots of key UI states and compares them to baselines
 * stored in `tests/e2e/spec/visual-regression.spec.ts-snapshots/`.
 *
 * First run creates baselines; subsequent runs diff against them.
 * Update baselines after intentional UI changes with:
 *
 *   npx playwright test visual-regression --update-snapshots
 *
 * CI does not fail the build on first runs — baselines get created
 * and reviewed in a follow-up PR.
 *
 * Run with:
 *   npx playwright test visual-regression
 */

import { test, expect } from '@playwright/test';

test.describe('Visual regression', () => {
  test('home page layout', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('home.png', {
      fullPage: true,
      maxDiffPixelRatio: 0.01, // 1% tolerance
    });
  });

  test('sidebar layout', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    const sidebar = page.locator('[data-testid="sidebar"]');
    await expect(sidebar).toBeVisible();
    await expect(sidebar).toHaveScreenshot('sidebar.png');
  });

  test('page view with content (journal today)', async ({ page }) => {
    const today = new Date().toISOString().split('T')[0];
    await page.goto(`/journal/${today}`);
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('page-view.png', {
      fullPage: false,
    });
  });

  test('block row default state', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    const firstBlock = page.locator('[data-testid^="block-row"]').first();
    // Skip the test if no blocks exist on the home page yet.
    if ((await firstBlock.count()) === 0) {
      test.skip();
      return;
    }
    await expect(firstBlock).toHaveScreenshot('block-row-default.png');
  });

  test('dark mode', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    const themeToggle = page.locator('[data-testid="theme-toggle"]');
    if (await themeToggle.isVisible()) {
      await themeToggle.click();
      // Allow CSS transition to settle.
      await page.waitForTimeout(300);
    }
    await expect(page).toHaveScreenshot('dark-mode.png', {
      fullPage: false,
    });
  });

  test('mobile viewport (375x667)', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('mobile-home.png', {
      fullPage: true,
    });
  });
});
