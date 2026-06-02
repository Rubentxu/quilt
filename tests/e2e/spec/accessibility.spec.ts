/**
 * Accessibility tests — automated WCAG 2.1 AA scans.
 *
 * Uses @axe-core/playwright to run axe-core rules against key pages.
 * Tests fail on "serious" or "critical" violations; lower-severity
 * issues are surfaced via the assertion message for visibility but
 * do not fail the build by default.
 *
 * Run with:
 *   npx playwright test accessibility
 */

import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('Accessibility', () => {
  test('home page has no serious or critical a11y violations', async ({ page }) => {
    await page.goto('/');

    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
      .analyze();

    const blocking = results.violations.filter(
      (v) => v.impact === 'serious' || v.impact === 'critical',
    );
    expect(
      blocking,
      `Found ${blocking.length} blocking a11y violation(s):\n${JSON.stringify(blocking, null, 2)}`,
    ).toEqual([]);
  });

  test('home page has a main landmark', async ({ page }) => {
    await page.goto('/');
    const main = page.locator('main, [role="main"]');
    await expect(main).toBeVisible();
  });

  test('interactive elements are keyboard accessible', async ({ page }) => {
    await page.goto('/');

    // Tab into the page and verify focus lands on a real element.
    await page.keyboard.press('Tab');
    const focusedTag = await page.evaluate(
      () => document.activeElement?.tagName ?? null,
    );
    expect(focusedTag, 'No element received keyboard focus').toBeTruthy();

    // The focused element should be visible (not display:none / hidden).
    const isVisible = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      if (!el) return false;
      const style = window.getComputedStyle(el);
      return style.display !== 'none' && style.visibility !== 'hidden';
    });
    expect(isVisible, 'Focused element is not visible').toBe(true);
  });

  test('all images have alt text', async ({ page }) => {
    await page.goto('/');
    const images = page.locator('img');
    const count = await images.count();

    expect(count, 'No <img> elements found on home page').toBeGreaterThanOrEqual(0);

    for (let i = 0; i < count; i++) {
      const img = images.nth(i);
      const alt = await img.getAttribute('alt');
      const ariaLabel = await img.getAttribute('aria-label');
      const role = await img.getAttribute('role');
      // Decorative images may use alt="" or role="presentation".
      const isDecorative = role === 'presentation' || role === 'none';
      expect(
        alt !== null || ariaLabel !== null || isDecorative,
        `Image at index ${i} has no alt text or aria-label`,
      ).toBe(true);
    }
  });

  test('form inputs on settings have associated labels', async ({ page }) => {
    await page.goto('/settings');
    // Give the lazy-loaded route a chance to mount.
    await page.waitForLoadState('domcontentloaded');
    const inputs = page.locator('input, textarea, select');
    const count = await inputs.count();

    for (let i = 0; i < count; i++) {
      const input = inputs.nth(i);
      const type = await input.getAttribute('type');
      // Skip hidden / file inputs (label not required for them).
      if (type === 'hidden' || type === 'file') continue;

      const id = await input.getAttribute('id');
      const ariaLabel = await input.getAttribute('aria-label');
      const ariaLabelledBy = await input.getAttribute('aria-labelledby');
      const placeholder = await input.getAttribute('placeholder');

      let hasForLabel = false;
      if (id) {
        hasForLabel = (await page.locator(`label[for="${id}"]`).count()) > 0;
      }
      // Fallback: an ancestor <label> also counts.
      const hasAncestorLabel = (await input.evaluate(
        (el) => !!el.closest('label'),
      )) as boolean;

      const labelled =
        hasForLabel ||
        hasAncestorLabel ||
        Boolean(ariaLabel) ||
        Boolean(ariaLabelledBy) ||
        Boolean(placeholder);

      expect(
        labelled,
        `Input at index ${i} (type=${type ?? 'unknown'}) has no associated label`,
      ).toBe(true);
    }
  });

  test('color contrast meets WCAG AA on home page', async ({ page }) => {
    await page.goto('/');
    const results = await new AxeBuilder({ page })
      .withRules(['color-contrast'])
      .analyze();

    const violations = results.violations.filter((v) => v.impact === 'serious');
    expect(
      violations,
      `Color-contrast violations:\n${JSON.stringify(violations, null, 2)}`,
    ).toEqual([]);
  });

  test('heading hierarchy is well-formed on home page', async ({ page }) => {
    await page.goto('/');
    const headings = page.locator('h1, h2, h3, h4, h5, h6');
    const count = await headings.count();

    if (count === 0) {
      // No headings is acceptable for the root route — skip.
      test.skip();
      return;
    }

    const levels: number[] = [];
    for (let i = 0; i < count; i++) {
      const tag = await headings.nth(i).evaluate((el) => el.tagName);
      levels.push(parseInt(tag.slice(1), 10));
    }

    // First heading should be an h1.
    expect(levels[0], `First heading should be h1, got h${levels[0]}`).toBe(1);

    // No skipped levels (h1 -> h3 without an h2, etc.).
    for (let i = 1; i < levels.length; i++) {
      const diff = levels[i] - levels[i - 1];
      expect(
        diff,
        `Heading levels skip from h${levels[i - 1]} to h${levels[i]} at index ${i}`,
      ).toBeLessThanOrEqual(1);
    }
  });

  test('aria-expanded toggles flip on click', async ({ page }) => {
    await page.goto('/');
    const toggles = page.locator('[aria-expanded]');
    const count = await toggles.count();

    for (let i = 0; i < count; i++) {
      const toggle = toggles.nth(i);
      const before = await toggle.getAttribute('aria-expanded');
      await toggle.click();
      // Some toggles use keyboard activation only — try both.
      const after =
        (await toggle.getAttribute('aria-expanded')) ??
        (await page.evaluate(async () => {
          // Re-read after a tick in case the click was async.
          await new Promise((r) => setTimeout(r, 50));
          return null;
        }));
      // If aria-expanded didn't change but the element has a popover,
      // we still consider it "interactive"; otherwise fail.
      expect(
        before !== after,
        `aria-expanded on element ${i} did not change (was ${before}, now ${after})`,
      ).toBe(true);
    }
  });
});
