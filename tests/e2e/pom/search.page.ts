/**
 * Search Page Object
 *
 * Page object for interacting with the search page.
 */

import { type Page, type Locator, expect } from '@playwright/test';
import { BasePage } from './base.page';

export class SearchPage extends BasePage {
  readonly searchInput: Locator;
  readonly emptyState: Locator;

  constructor(page: Page) {
    super(page);
    this.searchInput = this.locator('[data-testid="search-input"]');
    this.emptyState = this.locator('.empty-state');
  }

  /**
   * Search for a term and wait for results
   */
  async search(term: string) {
    await this.searchInput.fill(term);
    await this.searchInput.press('Enter');
  }

  /**
   * Wait for search results to appear
   */
  async waitForResults(timeout = 10000) {
    await this.page.waitForSelector('.search-results, .empty-state', { timeout });
  }

  /**
   * Get the empty state message
   */
  async getEmptyStateMessage(): Promise<string> {
    return this.emptyState.textContent() ?? '';
  }

  /**
   * Clear the search input
   */
  async clearSearch() {
    await this.searchInput.clear();
  }

  /**
   * Press arrow down in search results (keyboard navigation)
   */
  async pressArrowDown() {
    await this.searchInput.press('ArrowDown');
  }

  /**
   * Press arrow up in search results
   */
  async pressArrowUp() {
    await this.searchInput.press('ArrowUp');
  }

  /**
   * Press Enter to confirm selection
   */
  async pressEnter() {
    await this.searchInput.press('Enter');
  }
}
