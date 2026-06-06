/**
 * Navigation E2E Tests
 *
 * Tests for sidebar navigation, mobile menu, deep linking, and browser navigation.
 * Run with: npx playwright test navigation
 */

import { test, expect } from '@playwright/test';
import { SidebarComponent } from '../pom/sidebar.component';

test.describe('Mobile Sidebar', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:1420/');
  });

  test('mobile sidebar opens when menu button is clicked', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    // Click mobile menu button
    await page.click('[data-testid="mobile-menu-button"]');

    // Overlay should be visible
    await expect(page.locator('.mobile-sidebar-overlay')).toBeVisible();
  });

  test('mobile sidebar closes on backdrop click', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    // Open mobile menu
    await page.click('[data-testid="mobile-menu-button"]');
    await expect(page.locator('.mobile-sidebar-overlay')).toBeVisible();

    // Click backdrop to close
    await page.click('.mobile-sidebar-overlay');

    // Menu should close
    await expect(page.locator('.mobile-sidebar-overlay')).not.toBeVisible();
  });

  test('mobile sidebar closes on menu item click', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    // Open mobile menu
    await page.click('[data-testid="mobile-menu-button"]');
    await expect(page.locator('.mobile-sidebar-overlay')).toBeVisible();

    // Click a nav item
    await page.click('[data-testid="nav-pages"]');

    // Menu should close and we should be on pages
    await expect(page.locator('.mobile-sidebar-overlay')).not.toBeVisible();
    await expect(page).toHaveURL(/\/pages/);
  });

  test('mobile menu button shows close icon when open', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    // Initially shows hamburger
    const menuButton = page.locator('[data-testid="mobile-menu-button"]');
    await expect(menuButton).toContainText('☰');

    // Open menu
    await menuButton.click();

    // Now shows close icon
    await expect(menuButton).toContainText('×');
  });
});

test.describe('Active Route Highlighting', () => {
  const routes = [
    { path: '/journal', testId: 'nav-journal' },
    { path: '/pages', testId: 'nav-pages' },
    { path: '/graph', testId: 'nav-graph' },
  ];

  for (const route of routes) {
    test(`active route "${route.testId}" is highlighted on ${route.path}`, async ({ page }) => {
      await page.goto(`http://localhost:1420${route.path}`);
      await expect(page.locator(`[data-testid="${route.testId}"]`)).toHaveClass(/active/);
    });
  }

  test('journal is active on root path', async ({ page }) => {
    await page.goto('http://localhost:1420/');
    await expect(page.locator('[data-testid="nav-journal"]')).toHaveClass(/active/);
  });

  test('other routes are not active when on journal', async ({ page }) => {
    await page.goto('http://localhost:1420/journal');

    const sidebar = new SidebarComponent(page);
    await sidebar.expectActiveItem('nav-journal');
    await sidebar.expectNotActiveItem('nav-pages');
            await sidebar.expectNotActiveItem('nav-graph');
      });
});

test.describe('Deep Linking', () => {
  const routes = [
    { path: '/journal', heading: 'Journal' },
    { path: '/pages', heading: 'Pages' },
    { path: '/graph', heading: 'Vista Grafo' },
  ];

  for (const route of routes) {
    test(`direct navigation to ${route.path} works`, async ({ page }) => {
      await page.goto(`http://localhost:1420${route.path}`);
      await expect(page.locator(`h2:has-text("${route.heading}")`)).toBeVisible({ timeout: 10000 });
    });
  }

  test('journal with date parameter works', async ({ page }) => {
    await page.goto('http://localhost:1420/journal/2024-01-15');
    // Should load journal view without error
    await expect(page.locator('.journal-date, h2:has-text("Journal")')).toBeVisible({ timeout: 10000 });
  });
});

test.describe('Browser Back/Forward Navigation', () => {
  test('back button returns to previous route', async ({ page }) => {
    await page.goto('http://localhost:1420/');
    await expect(page.locator('[data-testid="nav-journal"]')).toHaveClass(/active/);

    // Navigate to Pages
    await page.click('[data-testid="nav-pages"]');
    await expect(page).toHaveURL(/\/pages/);

    // Go back
    await page.goBack();
    await expect(page).toHaveURL(/\/(journal)?$/);

    // Journal should be active again
    await expect(page.locator('[data-testid="nav-journal"]')).toHaveClass(/active/);
  });

  test('forward button advances to next route', async ({ page }) => {
    await page.goto('http://localhost:1420/');

    // Navigate to Pages
    await page.click('[data-testid="nav-pages"]');
    await expect(page).toHaveURL(/\/pages/);

    // Go back
    await page.goBack();

    // Go forward
    await page.goForward();
    await expect(page).toHaveURL(/\/pages/);
    await expect(page.locator('[data-testid="nav-pages"]')).toHaveClass(/active/);
  });

  test('clicking multiple nav items maintains history', async ({ page }) => {
    await page.goto('http://localhost:1420/');

    // Navigate through several pages
    await page.click('[data-testid="nav-pages"]');
    await page.click('[data-testid="nav-pages"]');
    await page.click('[data-testid="nav-query"]');

    await expect(page).toHaveURL(/\/query/);

    // Go back multiple times
    await page.goBack();
    await expect(page).toHaveURL(/\/search/);

    await page.goBack();
    await expect(page).toHaveURL(/\/pages/);

    await page.goBack();
    await expect(page).toHaveURL(/\/(journal)?$/);
  });
});
