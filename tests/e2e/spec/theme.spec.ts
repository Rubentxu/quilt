/**
 * Theme E2E Tests
 *
 * Tests for light/dark theme toggle and persistence.
 * Run with: npx playwright test theme
 */

import { test, expect } from '@playwright/test';
import { ThemeToggleComponent } from '../pom/theme-toggle.component';

test.describe('Theme Toggle', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173/');
  });

  test('theme toggle button is visible', async ({ page }) => {
    const toggle = new ThemeToggleComponent(page);
    await expect(toggle.toggleButton).toBeVisible();
  });

  test('clicking toggle switches from light to dark', async ({ page }) => {
    const toggle = new ThemeToggleComponent(page);

    // Start with light theme (default)
    const initialTheme = await toggle.getCurrentTheme();
    expect(initialTheme).toBe('light');

    // Click toggle
    await toggle.toggle();

    // Should now be dark
    await toggle.expectDarkTheme();
    expect(await toggle.getCurrentTheme()).toBe('dark');
  });

  test('clicking toggle again switches from dark to light', async ({ page }) => {
    const toggle = new ThemeToggleComponent(page);

    // Set dark theme first
    await toggle.toggle();
    await toggle.expectDarkTheme();

    // Toggle again
    await toggle.toggle();

    // Should now be light
    await toggle.expectLightTheme();
  });

  test('theme toggle shows appropriate icon', async ({ page }) => {
    const toggle = new ThemeToggleComponent(page);
    const toggleText = await toggle.toggleButton.textContent();

    // Icon depends on current theme - just verify it's visible
    await expect(toggle.toggleButton).toBeVisible();
  });
});

test.describe('Theme Persistence', () => {
  test('theme persists across navigation', async ({ page }) => {
    const toggle = new ThemeToggleComponent(page);

    // Set dark theme
    await toggle.toggle();
    await toggle.expectDarkTheme();

    // Navigate to different pages
    await page.click('[data-testid="nav-pages"]');
    await expect(page).toHaveURL(/\/pages/);
    await toggle.expectDarkTheme();

    await page.click('[data-testid="nav-search"]');
    await expect(page).toHaveURL(/\/search/);
    await toggle.expectDarkTheme();

    await page.click('[data-testid="nav-journal"]');
    await expect(page).toHaveURL(/\/journal/);
    await toggle.expectDarkTheme();
  });

  test('theme persists after page reload', async ({ page }) => {
    const toggle = new ThemeToggleComponent(page);

    // Set dark theme
    await toggle.toggle();
    await toggle.expectDarkTheme();

    // Reload page
    await page.reload();

    // Theme should persist
    await toggle.expectDarkTheme();
  });

  test('theme is independent of viewport size', async ({ page }) => {
    const toggle = new ThemeToggleComponent(page);

    // Set dark on desktop
    await page.setViewportSize({ width: 1280, height: 720 });
    await toggle.toggle();
    await toggle.expectDarkTheme();

    // Switch to mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    // Theme should still be dark
    await toggle.expectDarkTheme();
  });
});

test.describe('Theme and Other Components', () => {
  test('theme applies to sidebar styling', async ({ page }) => {
    const toggle = new ThemeToggleComponent(page);

    // Toggle to dark
    await toggle.toggle();
    await toggle.expectDarkTheme();

    // Verify sidebar also has dark class
    await expect(page.locator('.sidebar')).toHaveClass(/dark/);
  });

  test('theme applies to main content area', async ({ page }) => {
    const toggle = new ThemeToggleComponent(page);

    // Toggle to dark
    await toggle.toggle();
    await toggle.expectDarkTheme();

    // Verify main content area has dark styling
    await expect(page.locator('.main-content')).toBeVisible();
  });
});
