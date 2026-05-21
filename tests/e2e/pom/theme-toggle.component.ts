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
    const classList = await html.getAttribute('class');
    if (classList?.includes('dark')) {
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
    await expect(this.page.locator('html')).toHaveClass(/dark/);
  }

  /**
   * Expect theme to be light
   */
  async expectLightTheme() {
    const htmlClass = await this.page.locator('html').getAttribute('class');
    // Light theme has no dark class
    expect(htmlClass).not.toContain('dark');
  }
}
