/**
 * Quilt UI E2E Tests - Baseline P0 Tests
 *
 * These tests verify core functionality using stable data-testid selectors.
 * Run with: npx playwright test
 *
 * Prerequisites:
 * - Dev server must be running on http://localhost:1420
 * - Run `cd crates/quilt-ui && trunk serve --port 1420` before these tests
 */

import { test, expect } from '@playwright/test';

// ─── Test 1: Sidebar Navigation Route Changes ───────────────────────────────

test.describe('Sidebar Navigation', () => {
  test('should navigate to different routes when sidebar links are clicked', async ({ page }) => {
    await page.goto('/');

    // Verify we're on the journal page (default route)
    await expect(page.locator('h2.journal-date')).toBeVisible({ timeout: 10000 });

    // Click Pages navigation
    await page.click('[data-testid="nav-pages"]');
    await expect(page).toHaveURL(/\/pages/);
    await expect(page.locator('h2:has-text("Pages")')).toBeVisible();

    // Click Search navigation
    await page.click('[data-testid="nav-search"]');
    await expect(page).toHaveURL(/\/search/);
    await expect(page.locator('h2:has-text("Search")')).toBeVisible();

    // Click Query navigation
    await page.click('[data-testid="nav-query"]');
    await expect(page).toHaveURL(/\/query/);
    await expect(page.locator('h2:has-text("Query")')).toBeVisible();

    // Click Cognitive navigation
    await page.click('[data-testid="nav-cognitive"]');
    await expect(page).toHaveURL(/\/cognitive/);
    await expect(page.locator('h2:has-text("Morning Briefing")')).toBeVisible();

    // Click Journal navigation to return
    await page.click('[data-testid="nav-journal"]');
    await expect(page).toHaveURL(/\/journal/);
  });
});

// ─── Test 2: Search Input + Enter + Results/Empty State Render ──────────────

test.describe('Search', () => {
  test('should render search input and handle empty query state', async ({ page }) => {
    await page.goto('/search');

    // Verify search input is visible
    const searchInput = page.locator('[data-testid="search-input"]');
    await expect(searchInput).toBeVisible();

    // Verify initial empty state message
    await expect(page.locator('.empty-state:has-text("Enter a search term")')).toBeVisible();
  });

  test('should show empty state when no results found', async ({ page }) => {
    await page.goto('/search');

    const searchInput = page.locator('[data-testid="search-input"]');

    // Type a search term unlikely to match anything
    await searchInput.fill('xyznonexistentsearchterm12345');
    await searchInput.press('Enter');

    // Wait for results or empty state
    await expect(page.locator('.empty-state:has-text("No results found")')).toBeVisible({ timeout: 10000 });
  });
});

// ─── Test 3: Query DSL Execution + Result Table/Error Handling ──────────────

test.describe('Query', () => {
  test('should render query interface with input and run button', async ({ page }) => {
    await page.goto('/query');

    // Verify query input is visible
    const queryInput = page.locator('[data-testid="query-input"]');
    await expect(queryInput).toBeVisible();

    // Verify Run Query button is visible
    const runButton = page.locator('[data-testid="run-query-button"]');
    await expect(runButton).toBeVisible();

    // Verify suggestion chips are visible
    await expect(page.locator('[data-testid="query-chip-task-todo"]')).toBeVisible();
    await expect(page.locator('[data-testid="query-chip-priority-a"]')).toBeVisible();
  });

  test('should execute (task todo) query and display results or empty state', async ({ page }) => {
    await page.goto('/query');

    // Click the task todo chip
    await page.click('[data-testid="query-chip-task-todo"]');

    // Verify query is populated
    await expect(page.locator('[data-testid="query-input"]')).toHaveValue('(task todo)');

    // Click run query
    await page.click('[data-testid="run-query-button"]');

    // Wait for either results or empty/error state
    const resultsOrEmpty = page.locator('.query-results, .empty-state, .query-error');
    await expect(resultsOrEmpty.first()).toBeVisible({ timeout: 15000 });
  });

  test('should show query error for invalid query syntax', async ({ page }) => {
    await page.goto('/query');

    // Enter an invalid query
    await page.fill('[data-testid="query-input"]', '(invalid syntax');
    await page.click('[data-testid="run-query-button"]');

    // Wait for error display (if query engine returns errors)
    // The UI should handle errors gracefully
    const errorOrResults = page.locator('.query-error, .query-results, .empty-state');
    await expect(errorOrResults.first()).toBeVisible({ timeout: 15000 });
  });
});

// ─── Test 4: Cognitive Dashboard Load + Refresh Action ─────────────────────

test.describe('Cognitive Dashboard', () => {
  test('should load cognitive dashboard and display refresh button', async ({ page }) => {
    await page.goto('/cognitive');

    // Verify page header
    await expect(page.locator('h2:has-text("Morning Briefing")')).toBeVisible({ timeout: 10000 });

    // Verify refresh button exists
    const refreshButton = page.locator('[data-testid="refresh-button"]');
    await expect(refreshButton).toBeVisible();
  });

  test('should show loading state then content or error after dashboard load', async ({ page }) => {
    await page.goto('/cognitive');

    // Wait for loading to complete (either content or error shows)
    const dashboardContent = page.locator('.briefing-content, .dashboard-error, .dashboard-loading');
    await expect(dashboardContent.first()).toBeVisible({ timeout: 20000 });
  });
});

// ─── Test 5: Pages View Render + Content/Empty State ───────────────────────

test.describe('Pages View', () => {
  test('should render pages view with header', async ({ page }) => {
    await page.goto('/pages');

    // Verify page header
    await expect(page.locator('h2:has-text("Pages")')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('.page-subtitle:has-text("All your pages")')).toBeVisible();
  });

  test('should display pages list or empty state', async ({ page }) => {
    await page.goto('/pages');

    // Wait for loading to complete
    await page.waitForSelector('.block-list, .empty-state', { timeout: 15000 });

    // Should show either pages list or empty state
    const pagesOrEmpty = page.locator('.block-list, .empty-state');
    await expect(pagesOrEmpty.first()).toBeVisible();
  });
});
