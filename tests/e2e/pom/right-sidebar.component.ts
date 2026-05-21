/**
 * Right Sidebar Component Object
 *
 * Page object for interacting with the right sidebar.
 */

import { type Page, type Locator, expect } from '@playwright/test';
import { BasePage } from './base.page';

export class RightSidebarComponent extends BasePage {
  readonly propertiesTab: Locator;
  readonly backlinksTab: Locator;
  readonly annotationsTab: Locator;
  readonly closeButton: Locator;
  readonly sidebarPanel: Locator;

  constructor(page: Page) {
    super(page);
    this.propertiesTab = this.locator('[data-testid="tab-properties"]');
    this.backlinksTab = this.locator('[data-testid="tab-backlinks"]');
    this.annotationsTab = this.locator('[data-testid="tab-annotations"]');
    this.closeButton = this.locator('.right-sidebar-close');
    this.sidebarPanel = this.locator('.right-sidebar');
  }

  /**
   * Open the right sidebar (click the toggle)
   */
  async open() {
    const toggle = this.locator('.right-sidebar-toggle-btn');
    if (await toggle.isVisible()) {
      await toggle.click();
    }
  }

  /**
   * Close the right sidebar
   */
  async close() {
    await this.closeButton.click();
  }

  /**
   * Switch to properties tab
   */
  async switchToProperties() {
    await this.propertiesTab.click();
  }

  /**
   * Switch to backlinks tab
   */
  async switchToBacklinks() {
    await this.backlinksTab.click();
  }

  /**
   * Switch to annotations tab
   */
  async switchToAnnotations() {
    await this.annotationsTab.click();
  }

  /**
   * Check if sidebar is visible
   */
  async isVisible(): Promise<boolean> {
    return this.sidebarPanel.isVisible();
  }

  /**
   * Expect a specific tab to be active
   */
  async expectActiveTab(testId: string) {
    await expect(this.locator(`[data-testid="${testId}"]`)).toHaveClass(/active/);
  }
}
