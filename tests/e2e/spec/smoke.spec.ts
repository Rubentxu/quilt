/**
 * Smoke E2E Tests
 *
 * Verifies the React app shell loads correctly — sidebar, topbar,
 * and basic DOM structure. These should pass without a running backend.
 * Run with: npx playwright test smoke
 */

import { test, expect } from '@playwright/test';
import { ThemeToggleComponent } from '../pom/theme-toggle.component';

test.describe('App Shell', () => {
  test('page loads and shows app shell', async ({ page }) => {
    await page.goto('http://localhost:5173/');

    // React mounts into #root
    const root = page.locator('#root');
    await expect(root).toBeVisible({ timeout: 15_000 });

    // App shell container with data-testid
    const appShell = page.locator('[data-testid="app-shell"]');
    await expect(appShell).toBeVisible({ timeout: 10_000 });
  });

  test('sidebar is visible with navigation items', async ({ page }) => {
    await page.goto('http://localhost:5173/');

    // Sidebar container
    const sidebar = page.locator('[data-testid="sidebar"]');
    await expect(sidebar).toBeVisible({ timeout: 10_000 });

    // Navigation items should be present by data-testid
    await expect(page.locator('[data-testid="nav-journal"]')).toBeVisible({ timeout: 5_000 });
    await expect(page.locator('[data-testid="nav-pages"]')).toBeVisible();
    await expect(page.locator('[data-testid="nav-graph"]')).toBeVisible();
  });

  test('topbar shows breadcrumb', async ({ page }) => {
    await page.goto('http://localhost:5173/');

    // Breadcrumb should be visible
    const breadcrumb = page.locator('[data-testid="breadcrumb"]');
    await expect(breadcrumb).toBeVisible({ timeout: 10_000 });
    // On root path, breadcrumb shows "Home"
    await expect(breadcrumb).toContainText('Home');
  });

  test('theme toggle button is visible', async ({ page }) => {
    await page.goto('http://localhost:5173/');

    const toggle = new ThemeToggleComponent(page);
    await expect(toggle.toggleButton).toBeVisible({ timeout: 10_000 });
  });
});

test.describe('Theme Toggle', () => {
  test('clicking toggle switches theme', async ({ page }) => {
    await page.goto('http://localhost:5173/');

    const toggle = new ThemeToggleComponent(page);
    const initialTheme = await toggle.getCurrentTheme();

    // Toggle
    await toggle.toggle();

    // Theme should be different now
    const newTheme = await toggle.getCurrentTheme();
    expect(newTheme).not.toBe(initialTheme);
  });

  test('theme persists after page reload', async ({ page }) => {
    await page.goto('http://localhost:5173/');

    const toggle = new ThemeToggleComponent(page);

    // Toggle to dark
    await toggle.toggle();
    await toggle.expectDarkTheme();

    // Reload
    await page.reload();

    // Should still be dark
    await toggle.expectDarkTheme();
  });
});
