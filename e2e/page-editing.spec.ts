import { test, expect, type Page } from "@playwright/test";

const BASE_URL = process.env.BASE_URL || "http://localhost:3737";

let pageCounter = 0;

function uniquePageName(): string {
  pageCounter++;
  return `playwright-page-${Date.now()}-${pageCounter}`;
}

async function createBlock(page: Page, pageName: string, content: string) {
  const resp = await page.request.post(`${BASE_URL}/api/v1/blocks`, {
    data: { pageName, content, parentId: null },
  });
  expect(resp.ok()).toBeTruthy();
}

async function focusEditor(page: Page) {
  await page.evaluate(
    () =>
      (document.querySelector(
        ".cm6-editor-container [contenteditable]",
      ) as HTMLElement | null)?.focus(),
  );
}

test.describe("regular page editing", () => {
  test("single click enters edit mode and typing persists after reload", async ({ page }) => {
    const pageName = uniquePageName();
    await createBlock(page, pageName, "Regular page text");

    await page.goto(`${BASE_URL}/page/${pageName}`);
    await page.waitForTimeout(2000);

    const main = page.locator("main");
    await expect(main).toContainText("Regular page text");

    const blockText = page.locator("main").getByText("Regular page text");
    await blockText.first().click();
    await page.waitForTimeout(1000);

    await expect(page.locator(".cm6-editor-container")).toBeVisible({
      timeout: 10_000,
    });

    await focusEditor(page);
    await page.keyboard.type(" CLICKONLY ");
    await page.waitForTimeout(300);

    await page.getByRole("heading", { name: "Quilt" }).click();
    await page.waitForTimeout(2000);

    await expect(main).toContainText("CLICKONLY");

    await page.reload();
    await page.waitForTimeout(3000);

    await expect(main).toContainText("CLICKONLY");
  });

  test("Enter key splits block and persists both parts", async ({ page }) => {
    const pageName = uniquePageName();
    await createBlock(page, pageName, "First Second");

    await page.goto(`${BASE_URL}/page/${pageName}`);
    await page.waitForTimeout(2000);

    const main = page.locator("main");
    await expect(main).toContainText("First Second");

    // Click on block to enter edit mode
    const blockText = page.locator("main").getByText("First Second");
    await blockText.first().click();
    await page.waitForTimeout(1500);

    // Focus and type to verify editor is active
    await focusEditor(page);
    await page.waitForTimeout(500);

    // Press Enter to split the block (at the space between "First" and "Second")
    await page.keyboard.press("End");
    await page.keyboard.press("Home");
    await page.keyboard.press("End");
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(2000);

    // Verify two blocks exist via API
    const resp = await page.request.get(`${BASE_URL}/api/v1/pages/${pageName}/blocks`);
    expect(resp.ok()).toBeTruthy();
    const blocks = (await resp.json()) as Array<{ id: string; content: string }>;
    expect(blocks.length).toBeGreaterThanOrEqual(2);

    // Cleanup
    for (const block of blocks) {
      await page.request.delete(`${BASE_URL}/api/v1/blocks/${block.id}`);
    }
  });
});
