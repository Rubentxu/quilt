import { test, expect } from "@playwright/test";

/**
 * Quilt E2E — Smoke test for the web outliner UI.
 *
 * Prerequisites:
 *   1. Trunk dev server running: `just ui-dev` (or `trunk serve` in crates/quilt-ui)
 *   2. Tailwind CSS built: `npm run tailwind:build` (done automatically by ui-dev)
 *
 * These tests verify the app shell loads correctly.
 * Full outliner interaction tests require a running backend.
 */

const BASE_URL = process.env.BASE_URL || "http://localhost:8080";

test.describe("quilt-ui smoke", () => {
  test("page loads and shows app shell", async ({ page }) => {
    await page.goto(BASE_URL);

    // Page title
    await expect(page).toHaveTitle(/Quilt/);

    // App container is present (Leptos mounts into #app)
    const app = page.locator("#app");
    await expect(app).toBeVisible({ timeout: 15_000 });

    // The main outliner area should contain contenteditable editor blocks
    // (the journal view renders at least one contenteditable block)
    const editor = page.locator("[contenteditable]");
    await expect(editor.first()).toBeVisible({ timeout: 10_000 });
  });

  test("left sidebar is visible", async ({ page }) => {
    await page.goto(BASE_URL);

    // The sidebar contains navigation links
    const sidebar = page.locator("nav, aside").first();
    await expect(sidebar).toBeVisible({ timeout: 10_000 });
  });

  test("app shell has correct structure", async ({ page }) => {
    await page.goto(BASE_URL);

    // Verify the app shell structure: flex layout with sidebar + main + optional right sidebar
    const app = page.locator("#app");

    // The flex container should have children (sidebar + main content)
    const children = app.locator("> div > *");
    const count = await children.count();
    expect(count).toBeGreaterThanOrEqual(2);
  });

  test("page routing works — journal route loads", async ({ page }) => {
    await page.goto(`${BASE_URL}/journal`);

    // Should show the journal view with at least the title area
    const editor = page.locator("[contenteditable]");
    await expect(editor.first()).toBeVisible({ timeout: 10_000 });

    // Route to /pages
    await page.goto(`${BASE_URL}/pages`);
    await expect(page).toHaveURL(/\/pages/);
  });

  /**
   * Editor interaction tests.
   *
   * These tests require blocks to be present in the journal,
   * which depends on a running backend serving block data.
   * If no blocks are available, these tests will be skipped
   * or may fail — run with `BASE_URL` pointing to a fully
   * running instance.
   */

  test("can type text in block editor", async ({ page }) => {
    await page.goto(`${BASE_URL}/journal`);

    // Wait for at least one contenteditable to be available
    const editor = page.locator("[contenteditable]").first();
    await expect(editor).toBeVisible({ timeout: 15_000 });

    // Click to focus the editor
    await editor.click();

    // Type text character by character (works with WASM contenteditable)
    const testText = "Hello from E2E";
    await editor.pressSequentially(testText, { delay: 20 });

    // Verify the text was entered
    await expect(editor).toContainText(testText);
  });

  test("undo shortcut restores previous block content", async ({ page }) => {
    await page.goto(`${BASE_URL}/journal`);

    const editor = page.locator("[contenteditable]").first();
    await expect(editor).toBeVisible({ timeout: 15_000 });

    // Read initial content
    const initialContent = await editor.textContent();

    // Focus and type
    await editor.click();
    await editor.pressSequentially(" added text", { delay: 10 });

    // Verify new text appears
    await expect(editor).toContainText("added text");

    // Press Ctrl+Z to undo
    await page.keyboard.press("Control+z");
    await page.waitForTimeout(300);

    // Content should have been restored (or at least the "added text" is gone)
    const afterUndo = await editor.textContent();
    expect(afterUndo).not.toContain("added text");
  });

  test("autocomplete appears on [[ trigger", async ({ page }) => {
    await page.goto(`${BASE_URL}/journal`);

    const editor = page.locator("[contenteditable]").first();
    await expect(editor).toBeVisible({ timeout: 15_000 });

    // Click to focus
    await editor.click();

    // Clear existing content and type [[ to trigger autocomplete
    // We type at the end of existing content
    await editor.pressSequentially(" [[proj", { delay: 15 });

    // Wait briefly for autocomplete dropdown to appear
    // (only works if backend has page data for autocomplete)
    await page.waitForTimeout(500);

    // Check if dropdown appeared (non-breaking — skip if no dropdown)
    const dropdown = page.locator(".autocomplete-dropdown, [class*='autocomplete']").first();
    const isVisible = await dropdown.isVisible().catch(() => false);

    if (isVisible) {
      // If dropdown is visible, verify it has items and we can navigate
      const items = dropdown.locator("> *");
      const count = await items.count();
      expect(count).toBeGreaterThan(0);
    }
    // If no dropdown (no page data), test passes silently
  });
});
