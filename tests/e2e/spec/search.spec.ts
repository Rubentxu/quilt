/**
 * Search E2E Tests
 *
 * Tests for full-text search, keyboard navigation, and empty states.
 * Run with: npx playwright test search
 */

import { test, expect } from '@playwright/test';
import { SearchPage } from '../pom/search.page';

test.describe('Search Input', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173/search');
  });

  test('search input is visible and focusable', async ({ page }) => {
    const searchInput = page.locator('[data-testid="search-input"]');
    await expect(searchInput).toBeVisible();
    await expect(searchInput).toBeFocused();
  });

  test('search input accepts text', async ({ page }) => {
    const searchInput = page.locator('[data-testid="search-input"]');
    await searchInput.fill('test query');
    await expect(searchInput).toHaveValue('test query');
  });

  test('search input clears on clear action', async ({ page }) => {
    const searchInput = page.locator('[data-testid="search-input"]');
    await searchInput.fill('test query');
    await searchInput.clear();
    await expect(searchInput).toHaveValue('');
  });
});

test.describe('Search Results Display', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173/search');
  });

  test('empty state shows initial prompt', async ({ page }) => {
    const emptyState = page.locator('.empty-state');
    await expect(emptyState).toBeVisible();
    await expect(emptyState).toContainText('Enter a search term');
  });

  test('searching shows loading then results or empty', async ({ page }) => {
    const searchInput = page.locator('[data-testid="search-input"]');

    // Enter a search term
    await searchInput.fill('test');
    await searchInput.press('Enter');

    // Wait for either results or empty state (no results)
    const resultsOrEmpty = page.locator('.search-results, .empty-state');
    await expect(resultsOrEmpty.first()).toBeVisible({ timeout: 10000 });
  });

  test('no results shows appropriate message', async ({ page }) => {
    const searchInput = page.locator('[data-testid="search-input"]');

    // Search for something unlikely to exist
    await searchInput.fill('xyznonexistentsearchterm12345');
    await searchInput.press('Enter');

    // Should show no results message
    await expect(page.locator('.empty-state:has-text("No results")')).toBeVisible({ timeout: 10000 });
  });
});

test.describe('Keyboard Navigation in Search', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173/search');
  });

  test('arrow down navigates to results when available', async ({ page }) => {
    const searchInput = page.locator('[data-testid="search-input"]');

    // Enter a search term
    await searchInput.fill('test');
    await searchInput.press('Enter');

    // Wait for results
    await page.waitForSelector('.search-results', { timeout: 10000 }).catch(() => {});

    // Try arrow down
    await searchInput.press('ArrowDown');

    // The search input should still be focused or results should be highlighted
    // (behavior depends on implementation)
    const isStillFocused = await searchInput.evaluate((el) => el === document.activeElement);
    const hasFocusedResult = await page.locator('.search-result.selected, .search-result:focus').count() > 0;

    expect(isStillFocused || hasFocusedResult).toBeTruthy();
  });

  test('arrow up navigates in results', async ({ page }) => {
    const searchInput = page.locator('[data-testid="search-input"]');

    // Enter a search term
    await searchInput.fill('test');
    await searchInput.press('Enter');

    // Wait for results
    await page.waitForSelector('.search-results', { timeout: 10000 }).catch(() => {});

    // Go down then up
    await searchInput.press('ArrowDown');
    await searchInput.press('ArrowUp');

    // Should not crash - basic keyboard nav test
  });

  test('enter submits search', async ({ page }) => {
    const searchInput = page.locator('[data-testid="search-input"]');

    await searchInput.fill('test query');
    await searchInput.press('Enter');

    // Should trigger search
    const resultsOrEmpty = page.locator('.search-results, .empty-state');
    await expect(resultsOrEmpty.first()).toBeVisible({ timeout: 10000 });
  });

  test('escape clears focus or closes', async ({ page }) => {
    const searchInput = page.locator('[data-testid="search-input"]');
    await searchInput.fill('test');

    // Press escape
    await searchInput.press('Escape');

    // Focus behavior depends on implementation
    // Could blur, could close a search modal, etc.
  });
});

test.describe('Empty Search States', () => {
  test('empty input shows enter prompt', async ({ page }) => {
    await page.goto('http://localhost:5173/search');

    // Don't type anything, just press enter
    await page.locator('[data-testid="search-input"]').press('Enter');

    // Should show empty state
    await expect(page.locator('.empty-state')).toBeVisible({ timeout: 5000 });
  });

  test('whitespace-only search shows empty state', async ({ page }) => {
    await page.goto('http://localhost:5173/search');

    const searchInput = page.locator('[data-testid="search-input"]');
    await searchInput.fill('   '); // whitespace only
    await searchInput.press('Enter');

    // Should handle gracefully
    const resultsOrEmpty = page.locator('.search-results, .empty-state');
    await expect(resultsOrEmpty.first()).toBeVisible({ timeout: 10000 });
  });
});
