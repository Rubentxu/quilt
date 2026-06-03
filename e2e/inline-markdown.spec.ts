import { test, expect, type Page } from "@playwright/test";

/**
 * Quilt E2E — Inline Markdown Rendering tests.
 *
 * Prerequisites:
 *   1. Full server running on port 3737 (just build-all && just server-dev)
 *   2. BASE_URL env or default http://localhost:3737
 *
 * These tests verify that block content with inline markdown syntax
 * renders as proper HTML elements in display (non-editing) mode:
 * - **bold** → <strong>
 * - *italic* → <em>
 * - `code` → <code>
 * - [text](url) → <a href>
 * - Does NOT interfere with [[page refs]], ((block refs)), #tags, property:: value
 */

const BASE_URL = process.env.BASE_URL || "http://localhost:3737";

let dateCounter = 0;

/** Returns a unique date string so tests never share a journal page. */
function uniqueDate(): string {
  dateCounter++;
  const d = new Date();
  d.setDate(d.getDate() + 180 + dateCounter);
  return d.toISOString().slice(0, 10);
}

/** Create a block via the REST API and return its ID. */
async function createBlock(
  page: Page,
  date: string,
  content: string,
  parentId: string | null = null,
): Promise<string> {
  const resp = await page.request.post(`${BASE_URL}/api/v1/blocks`, {
    data: { pageName: date, content, parentId },
  });
  expect(resp.ok()).toBeTruthy();
  return ((await resp.json()) as { id: string }).id;
}

/** Remove all blocks from a journal page so the test starts clean. */
async function deleteAllBlocks(page: Page, date: string) {
  const resp = await page.request.get(
    `${BASE_URL}/api/v1/pages/${date}/blocks`,
  );
  expect(resp.ok()).toBeTruthy();
  const blocks = (await resp.json()) as Array<{ id: string }>;
  for (const block of blocks) {
    const del = await page.request.delete(
      `${BASE_URL}/api/v1/blocks/${block.id}`,
    );
    expect(del.ok()).toBeTruthy();
  }
}

test.describe("inline markdown rendering", () => {
  test("**bold** renders as <strong> in display mode", async ({ page }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    await createBlock(page, date, "This is **important** text");
    await page.waitForTimeout(1500);

    // The strong element should contain "important"
    const strong = page.locator("strong").first();
    await expect(strong).toBeVisible({ timeout: 10_000 });
    await expect(strong).toHaveText("important");
  });

  test("*italic* renders as <em> in display mode", async ({ page }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    await createBlock(page, date, "This is *emphasized* text");
    await page.waitForTimeout(1500);

    const em = page.locator("em").first();
    await expect(em).toBeVisible({ timeout: 10_000 });
    await expect(em).toHaveText("emphasized");
  });

  test("`code` renders as <code> in display mode", async ({ page }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    await createBlock(page, date, "Use the `print()` function");
    await page.waitForTimeout(1500);

    const code = page.locator("code").first();
    await expect(code).toBeVisible({ timeout: 10_000 });
    await expect(code).toHaveText("print()");
  });

  test("[text](url) renders as <a> in display mode", async ({ page }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    await createBlock(page, date, "Visit [Quilt](https://quilt.dev)");
    await page.waitForTimeout(1500);

    const link = page.locator("a.cm-inline-link").first();
    await expect(link).toBeVisible({ timeout: 10_000 });
    await expect(link).toHaveText("Quilt");
    await expect(link).toHaveAttribute("href", "https://quilt.dev");
  });

  test("mixed bold, italic, and code render together", async ({ page }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    await createBlock(page, date, "**bold** and *italic* and `code`");
    await page.waitForTimeout(1500);

    await expect(page.locator("strong")).toBeVisible({ timeout: 10_000 });
    await expect(page.locator("em")).toBeVisible();
    await expect(page.locator("code")).toBeVisible();
  });

  test("inline markdown does not interfere with [[page refs]]", async ({
    page,
  }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    await createBlock(page, date, "See [[My Page]] for **details**");
    await page.waitForTimeout(1500);

    // Both the page ref and the bold should render
    const pageRef = page.locator(".decoration-page-ref").first();
    await expect(pageRef).toBeVisible({ timeout: 10_000 });

    const bold = page.locator("strong").first();
    await expect(bold).toBeVisible();
  });

  test("property value with markdown syntax is not rendered as markdown", async ({
    page,
  }) => {
    const date = uniqueDate();
    await deleteAllBlocks(page, date);
    await page.goto(`${BASE_URL}/journal/${date}`);
    await page.waitForTimeout(2000);

    // The property value **active** should display as plain text
    await createBlock(page, date, "status:: **active**");
    await page.waitForTimeout(1500);

    // The strong element should NOT exist since status value is not markdown-rendered
    const strong = page.locator("strong");
    await expect(strong).toHaveCount(0);
  });
});
