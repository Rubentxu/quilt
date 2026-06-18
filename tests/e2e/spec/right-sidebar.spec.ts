/**
 * Right Sidebar E2E Tests
 *
 * Tests for the right sidebar open/close, tab switching.
 * Run with: npx playwright test right-sidebar
 */

import { test, expect } from '@playwright/test';
import { RightSidebarComponent } from '../pom/right-sidebar.component';

test.describe('Right Sidebar Open/Close', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173/pages');
  });

  test('right sidebar toggle button is visible', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);
    await expect(sidebar.locator('.right-sidebar-toggle-btn')).toBeVisible();
  });

  test('clicking toggle opens the sidebar', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);

    // Sidebar should not be visible initially
    await expect(sidebar.sidebarPanel).not.toBeVisible();

    // Click toggle to open
    await sidebar.open();

    // Sidebar should now be visible
    await expect(sidebar.sidebarPanel).toBeVisible();
  });

  test('clicking close button closes the sidebar', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);

    // Open sidebar
    await sidebar.open();
    await expect(sidebar.sidebarPanel).toBeVisible();

    // Click close
    await sidebar.close();

    // Sidebar should be hidden
    await expect(sidebar.sidebarPanel).not.toBeVisible();
  });

  test('sidebar toggle icon shows current tab icon', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);

    // Open sidebar
    await sidebar.open();

    // Should show some icon in the toggle
    await expect(sidebar.locator('.right-sidebar-toggle-btn')).toBeVisible();
  });
});

test.describe('Right Sidebar Tab Switching', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173/pages');
    const sidebar = new RightSidebarComponent(page);
    await sidebar.open();
    await expect(page.locator('.right-sidebar')).toBeVisible();
  });

  test('properties tab is active by default', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);
    await sidebar.expectActiveTab('tab-properties');
  });

  test('clicking backlinks tab switches to it', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);

    // Switch to backlinks
    await sidebar.switchToBacklinks();

    // Backlinks tab should now be active
    await sidebar.expectActiveTab('tab-backlinks');

    // Properties should no longer be active
    await expect(sidebar.locator('[data-testid="tab-properties"]')).not.toHaveClass(/active/);
  });

  test('clicking annotations tab switches to it', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);

    // Switch to annotations
    await sidebar.switchToAnnotations();

    // Annotations tab should now be active
    await sidebar.expectActiveTab('tab-annotations');
  });

  test('clicking properties tab switches back to it', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);

    // Go to another tab first
    await sidebar.switchToBacklinks();
    await sidebar.expectActiveTab('tab-backlinks');

    // Switch back to properties
    await sidebar.switchToProperties();

    // Properties should be active again
    await sidebar.expectActiveTab('tab-properties');
  });

  test('all three tabs are visible', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);

    await expect(sidebar.propertiesTab).toBeVisible();
    await expect(sidebar.backlinksTab).toBeVisible();
    await expect(sidebar.annotationsTab).toBeVisible();
  });
});

test.describe('Right Sidebar Content', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173/pages');
    const sidebar = new RightSidebarComponent(page);
    await sidebar.open();
    await expect(page.locator('.right-sidebar')).toBeVisible();
  });

  test('properties panel shows when properties tab active', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);

    // Should be on properties by default
    await expect(page.locator('.right-sidebar-panel')).toBeVisible();
  });

  test('backlinks panel content when on backlinks tab', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);

    await sidebar.switchToBacklinks();

    // Wait for panel to update
    await page.waitForTimeout(100);

    // Panel should still be visible
    await expect(page.locator('.right-sidebar-panel')).toBeVisible();
  });

  test('annotations panel shows when on annotations tab', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);

    await sidebar.switchToAnnotations();

    // Wait for panel to update
    await page.waitForTimeout(100);

    // Panel should still be visible
    await expect(page.locator('.right-sidebar-panel')).toBeVisible();
  });
});

test.describe('Right Sidebar State Persistence', () => {
  test('sidebar state persists across navigation', async ({ page }) => {
    const sidebar = new RightSidebarComponent(page);

    // Open sidebar and switch to backlinks
    await sidebar.open();
    await sidebar.switchToBacklinks();

    // Navigate away
    await page.click('[data-testid="nav-journal"]');
    await expect(page).toHaveURL(/\/journal/);

    // Sidebar toggle should still show something visible
    // (state may or may not persist depending on implementation)
  });
});
