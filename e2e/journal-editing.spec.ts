import { test, expect, type Page } from "@playwright/test";

/**
 * Quilt E2E — Journal editing tests.
 *
 * Prerequisites:
 *   1. Full server running on port 3737 (just build-all && just server-dev)
 *   2. BASE_URL env or default http://localhost:3737
 *
 * These tests verify the full journal editing flow against a running backend.
 * Each test uses a unique date prefix so they never collide.
 */

const BASE_URL = process.env.BASE_URL || "http://localhost:3737";

let dateCounter = 0;

/** Returns a unique date string so tests never share a journal page. */
function uniqueDate(): string {
  dateCounter++;
  const d = new Date();
  d.setDate(d.getDate() + 90 + dateCounter);
  return d.toISOString().slice(0, 10);
}

/** Create a block via the REST API and return its ID. */
async function createBlock(page: Page, date: string, content: string) {
  const resp = await page.request.post(`${BASE_URL}/api/v1/blocks`, {
    data: { pageName: date, content, parentId: null },
  });
  expect(resp.ok()).toBeTruthy();
  return ((await resp.json()) as { id: string }).id;
}

/** Remove all blocks from a journal page so the test starts clean. */
async function deleteAllBlocks(page: Page, date: string) {
  const resp = await page.request.get(`${BASE_URL}/api/v1/pages/${date}/blocks`);
  expect(resp.ok()).toBeTruthy();
  const blocks = (await resp.json()) as Array<{ id: string }>;
  for (const block of blocks) {
    const del = await page.request.delete(`${BASE_URL}/api/v1/blocks/${block.id}`);
    expect(del.ok()).toBeTruthy();
  }
}

/**
 * Helper: focus the CM6 contenteditable inside the block editor.
 * In the intended UX, single click should already enter edit mode and focus.
 */
async function focusEditor(page: Page) {
  await page.evaluate(
    () =>
      (document.querySelector(
        ".cm6-editor-container [contenteditable]",
      ) as HTMLElement | null)?.focus(),
  );
}

test.describe("journal editing", () => {
  test("clicking 'No notes yet' creates the first block and auto-enters edit mode", async ({
    page,
  }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    const emptyMsg = page.getByText("No notes yet");
    await expect(emptyMsg).toBeVisible({ timeout: 10_000 });

    // Click — creates a block and opens CM6
    await emptyMsg.click();
    await page.waitForTimeout(1500);

    const editor = page.locator(".cm6-editor-container");
    await expect(editor).toBeVisible({ timeout: 10_000 });

    // Type content
    await focusEditor(page);
    await page.keyboard.type("Hello from first block! ");
    await page.waitForTimeout(300);

    // Blur to save
    await page.getByRole("heading", { name: "Quilt" }).click();
    await page.waitForTimeout(2000);

    const main = page.locator("main");
    await expect(main).toContainText("Hello from first block!");

    // Reload and verify persistence
    await page.reload();
    await page.waitForTimeout(3000);
    await expect(main).toContainText("Hello from first block!");
  });

  test("single click enters edit mode and edited block persists after reload", async ({ page }) => {
    const date = uniqueDate();
    await createBlock(page, date, "Original text");
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    const main = page.locator("main");
    await expect(main).toContainText("Original text");

    // Single click should start editing (Logseq-like UX)
    const blockText = page.locator("main").getByText("Original text");
    await blockText.first().click();
    await page.waitForTimeout(1000);

    await expect(page.locator(".cm6-editor-container")).toBeVisible({
      timeout: 10_000,
    });

    await focusEditor(page);
    await page.keyboard.type("UPDATED ");
    await page.waitForTimeout(300);

    // Blur to save
    await page.getByRole("heading", { name: "Quilt" }).click();
    await page.waitForTimeout(2000);

    await expect(main).toContainText("UPDATED");

    // Reload and verify persistence
    await page.reload();
    await page.waitForTimeout(3000);
    await expect(main).toContainText("UPDATED");
  });

  test("empty journal page shows 'No notes yet' message", async ({ page }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    await expect(page.getByText("No notes yet")).toBeVisible({
      timeout: 10_000,
    });

    // CM6 editor should NOT be present (nothing was clicked)
    await expect(page.locator(".cm6-editor-container")).toHaveCount(0);
  });

  test("saved content survives navigation away and back", async ({ page }) => {
    const date = uniqueDate();
    await createBlock(page, date, "Nav test");
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    // Edit the block
    const blockText = page.locator("main").getByText("Nav test");
    await blockText.first().click();
    await page.waitForTimeout(500);
    await page.keyboard.press("Enter");
    await page.waitForTimeout(1000);

    await focusEditor(page);
    await page.keyboard.type(" SAVED ");
    await page.waitForTimeout(200);

    // Blur to save BEFORE navigating
    await page.getByRole("heading", { name: "Quilt" }).click();
    await page.waitForTimeout(2000);

    const main = page.locator("main");
    await expect(main).toContainText("SAVED");

    // Navigate to another day via Previous day button
    await page.locator('button[title*="Previous day"]').click();
    await page.waitForTimeout(2000);

    // Navigate back via Next day button
    await page.locator('button[title*="Next day"]').click();
    await page.waitForTimeout(3000);

    // Content should still be there
    await expect(main).toContainText("SAVED");
  });

  test("single click opens block editor", async ({ page }) => {
    const date = uniqueDate();
    await createBlock(page, date, "ClickEnter");
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    const blockText = page.locator("main").getByText("ClickEnter");
    await blockText.first().click();
    await page.waitForTimeout(1000);

    await expect(page.locator(".cm6-editor-container")).toBeVisible({
      timeout: 10_000,
    });
  });
});
