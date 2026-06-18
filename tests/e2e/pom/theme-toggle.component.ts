/**
 * Theme Toggle Component Object
 *
 * Page object for interacting with theme toggle.
 */

import { type Page, type Locator, expect } from '@playwright/test';
import { BasePage } from './base.page';

export class ThemeToggleComponent extends BasePage {
  readonly toggleButton: Locator;

  constructor(page: Page) {
    super(page);
    this.toggleButton = this.locator('[data-testid="theme-toggle"]');
  }

  /**
   * Get current theme (returns 'light' or 'dark')
   */
  async getCurrentTheme(): Promise<'light' | 'dark'> {
    const html = this.page.locator('html');
    const dataTheme = await html.getAttribute('data-theme');
    if (dataTheme === 'dark') {
      return 'dark';
    }
    return 'light';
  }

  /**
   * Toggle the theme
   */
  async toggle() {
    await this.toggleButton.click();
  }

  /**
   * Expect theme to be dark
   */
  async expectDarkTheme() {
    await expect(this.page.locator('html')).toHaveAttribute('data-theme', 'dark');
  }

  /**
   * Expect theme to be light
   */
  async expectLightTheme() {
    await expect(this.page.locator('html')).toHaveAttribute('data-theme', 'light');
  }
}
