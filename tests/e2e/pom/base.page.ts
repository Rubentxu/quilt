/**
 * Base Page Object Model for Quilt UI E2E Tests
 *
 * Provides common functionality for all page objects.
 */

import { type Page, type Locator } from '@playwright/test';

export class BasePage {
  constructor(protected page: Page) {}

  /**
   * Navigate to a path within the app
   */
  async goto(path: string) {
    await this.page.goto(`http://localhost:5173${path}`);
  }

  /**
   * Wait for page to be fully loaded
   */
  async waitForLoad() {
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Get a locator for a selector
   */
  protected locator(selector: string): Locator {
    return this.page.locator(selector);
  }

  /**
   * Get current URL
   */
  async getCurrentUrl(): Promise<string> {
    return this.page.url();
  }

  /**
   * Check if element is visible
   */
  async isVisible(selector: string): Promise<boolean> {
    return this.locator(selector).isVisible();
  }

  /**
   * Click an element
   */
  async click(selector: string) {
    await this.locator(selector).click();
  }

  /**
   * Fill an input field
   */
  async fill(selector: string, value: string) {
    await this.locator(selector).fill(value);
  }

  /**
   * Get text content of an element
   */
  async getText(selector: string): Promise<string> {
    return this.locator(selector).textContent() ?? '';
  }
}
