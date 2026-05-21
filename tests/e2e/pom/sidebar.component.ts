/**
 * Sidebar Component Object
 *
 * Page object for interacting with the sidebar navigation.
 */

import { type Page, type Locator, expect } from '@playwright/test';
import { BasePage } from './base.page';

export class SidebarComponent extends BasePage {
  private navItems: Locator;

  constructor(page: Page) {
    super(page);
    this.navItems = this.locator('.sidebar-nav-item');
  }

  /**
   * Click a navigation item by its data-testid
   */
  async clickNavItem(testId: string) {
    await this.locator(`[data-testid="${testId}"]`).click();
  }

  /**
   * Expect a navigation item to have the active class
   */
  async expectActiveItem(testId: string) {
    await expect(this.locator(`[data-testid="${testId}"]`)).toHaveClass(/active/);
  }

  /**
   * Expect a navigation item to NOT have the active class
   */
  async expectNotActiveItem(testId: string) {
    await expect(this.locator(`[data-testid="${testId}"]`)).not.toHaveClass(/active/);
  }

  /**
   * Get all navigation item testids
   */
  async getNavItems(): Promise<string[]> {
    return this.navItems.evaluateAll((els) =>
      els.map((el) => el.getAttribute('data-testid') || '')
    );
  }

  /**
   * Check if mobile sidebar overlay is visible
   */
  async isMobileOverlayVisible(): Promise<boolean> {
    return this.locator('.mobile-sidebar-overlay').isVisible();
  }

  /**
   * Click mobile menu button (hamburger/close)
   */
  async clickMobileMenuButton() {
    await this.locator('[data-testid="mobile-menu-button"]').click();
  }
}
