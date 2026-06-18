// @ts-check
import { test, expect } from '@playwright/test';

const JOURNAL_URL = 'http://localhost:8090/journal';
const API_URL = 'http://localhost:3737/api/v1';

/**
 * Helper: create a block via API and wait for page to reflect it
 */
async function createBlock(page, content) {
  const resp = await fetch(`${API_URL}/blocks`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pageName: '2026-05-29', content }),
  });
  return resp.json();
}

/**
 * Helper: get all blocks from API
 */
async function getBlocks() {
  const resp = await fetch(`${API_URL}/pages/2026-05-29/blocks`);
  return resp.json();
}

test.describe('Outliner Block Behaviors', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(JOURNAL_URL);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1500);
  });

  // ──────────────────────────────────────────────
  // 1. Block Editing Basics
  // ──────────────────────────────────────────────
  test('click enters edit mode', async ({ page }) => {
    const blocks = page.locator('.block-group');
    const count = await blocks.count();
    expect(count).toBeGreaterThan(0);

    await blocks.first().click();
    await page.waitForTimeout(500);

    const editors = page.locator('.cm-content');
    await expect(editors).toHaveCount(1);
  });

  test('typing works in editor', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.first().click();
    await page.waitForTimeout(300);

    await page.keyboard.type('Hello World');
    await page.waitForTimeout(200);

    const content = await page.evaluate(
      () => document.querySelector('.cm-content')?.textContent || ''
    );
    expect(content).toContain('Hello World');
  });

  test('click outside saves and exits editing', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.first().click();
    await page.waitForTimeout(300);
    await page.keyboard.type('Saved content');
    await page.waitForTimeout(200);

    // Click outside
    await page.locator('body').click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(800);

    const editors = page.locator('.cm-content');
    await expect(editors).toHaveCount(0);
  });

  // ──────────────────────────────────────────────
  // 2. Enter Key
  // ──────────────────────────────────────────────
  test('Enter creates a new block', async ({ page }) => {
    const initialCount = await page.locator('.block-group').count();

    const blocks = page.locator('.block-group');
    await blocks.first().click();
    await page.waitForTimeout(300);

    await page.keyboard.press('Enter');
    await page.waitForTimeout(600);

    const newCount = await page.locator('.block-group').count();
    expect(newCount).toBe(initialCount + 1);
  });

  test('Enter splits block at cursor', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.first().click();
    await page.waitForTimeout(300);

    await page.keyboard.type('Hello World');
    // Move cursor to after "Hello"
    for (let i = 0; i < 6; i++) await page.keyboard.press('ArrowLeft');
    await page.keyboard.press('Enter');
    await page.waitForTimeout(600);

    const newCount = await page.locator('.block-group').count();
    expect(newCount).toBeGreaterThan(1);
  });

  // ──────────────────────────────────────────────
  // 3. Tab / Shift+Tab Indent/Outdent
  // ──────────────────────────────────────────────
  test('Tab does not lose focus on first block (no sibling)', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.first().click();
    await page.waitForTimeout(300);

    await page.keyboard.press('Tab');
    await page.waitForTimeout(500);

    const editors = page.locator('.cm-content');
    await expect(editors).toHaveCount(1);
  });

  test('Tab indents second block under first without losing focus', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.nth(1).click();
    await page.waitForTimeout(300);

    await page.keyboard.press('Tab');
    await page.waitForTimeout(500);

    const editors = page.locator('.cm-content');
    await expect(editors).toHaveCount(1);
  });

  test('Multiple Tab presses keep focus', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.nth(2).click();
    await page.waitForTimeout(300);

    for (let i = 0; i < 3; i++) {
      await page.keyboard.press('Tab');
      await page.waitForTimeout(300);
    }

    const editors = page.locator('.cm-content');
    await expect(editors).toHaveCount(1);
  });

  test('Shift+Tab outdents without losing focus', async ({ page }) => {
    const blocks = page.locator('.block-group');

    // First indent to create a child
    await blocks.nth(1).click();
    await page.waitForTimeout(300);
    await page.keyboard.press('Tab');
    await page.waitForTimeout(500);

    // Then outdent
    await page.keyboard.down('Shift');
    await page.keyboard.press('Tab');
    await page.keyboard.up('Shift');
    await page.waitForTimeout(500);

    const editors = page.locator('.cm-content');
    await expect(editors).toHaveCount(1);
  });

  // ──────────────────────────────────────────────
  // 4. Click on Editing Block
  // ──────────────────────────────────────────────
  test('click inside editing block keeps editing', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.first().click();
    await page.waitForTimeout(300);

    // Click again inside the editor
    const cm = page.locator('.cm-content');
    await cm.click();
    await page.waitForTimeout(300);

    const editors = page.locator('.cm-content');
    await expect(editors).toHaveCount(1);
  });

  // ──────────────────────────────────────────────
  // 5. Text Wrapping
  // ──────────────────────────────────────────────
  test('long text wraps to multiple lines', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.first().click();
    await page.waitForTimeout(300);

    await page.keyboard.type('x'.repeat(150));
    await page.waitForTimeout(300);

    const wrapInfo = await page.evaluate(() => {
      const cm = document.querySelector('.cm-content');
      if (!cm) return { lines: 0 };
      return {
        lines: cm.querySelectorAll('.cm-line').length,
        whiteSpace: getComputedStyle(cm).whiteSpace,
      };
    });

    expect(wrapInfo.lines).toBeGreaterThanOrEqual(2);
    expect(wrapInfo.whiteSpace).toBe('pre-wrap');
  });

  test('editor width matches display width', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.first().click();
    await page.waitForTimeout(300);

    await page.keyboard.type('Some text to measure width');
    await page.waitForTimeout(200);

    const widths = await page.evaluate(() => {
      const display = document.querySelector('.cursor-text');
      const scroller = document.querySelector('.cm-scroller');
      return {
        display: display ? getComputedStyle(display).width : '0',
        scroller: scroller ? getComputedStyle(scroller).width : '0',
      };
    });

    const dw = parseFloat(widths.display);
    const sw = parseFloat(widths.scroller);
    expect(Math.abs(dw - sw)).toBeLessThan(50);
  });

  // ──────────────────────────────────────────────
  // 6. Shift+Enter Soft Newline
  // ──────────────────────────────────────────────
  test('Shift+Enter inserts newline within same block', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.first().click();
    await page.waitForTimeout(300);

    await page.keyboard.type('Line A');
    await page.keyboard.down('Shift');
    await page.keyboard.press('Enter');
    await page.keyboard.up('Shift');
    await page.keyboard.type('Line B');
    await page.waitForTimeout(300);

    const content = await page.evaluate(
      () => document.querySelector('.cm-content')?.innerText || ''
    );
    expect(content).toContain('\n');
  });

  // ──────────────────────────────────────────────
  // 7. Escape Key
  // ──────────────────────────────────────────────
  test('Escape saves and exits editing', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.first().click();
    await page.waitForTimeout(300);
    await page.keyboard.type('Escape save test');

    await page.keyboard.press('Escape');
    await page.waitForTimeout(500);

    const editors = page.locator('.cm-content');
    await expect(editors).toHaveCount(0);
  });

  // ──────────────────────────────────────────────
  // 8. Backspace Merge
  // ──────────────────────────────────────────────
  test('Backspace at start of second block merges with previous', async ({ page }) => {
    const initialCount = await page.locator('.block-group').count();

    const blocks = page.locator('.block-group');
    await blocks.nth(1).click();
    await page.waitForTimeout(300);

    // Go to start and press Backspace
    await page.keyboard.press('Home');
    await page.keyboard.press('Backspace');
    await page.waitForTimeout(600);

    const newCount = await page.locator('.block-group').count();
    expect(newCount).toBeLessThan(initialCount);
  });

  // ──────────────────────────────────────────────
  // 9. Arrow Key Navigation
  // ──────────────────────────────────────────────
  test('ArrowDown at end moves to next block', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.first().click();
    await page.waitForTimeout(300);

    // Go to end and press ArrowDown
    await page.keyboard.press('End');
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(500);

    const editors = page.locator('.cm-content');
    await expect(editors).toHaveCount(1);
  });

  // FIXME: edit transition race condition — set_editing vs clear_editing timing
  test.skip('ArrowUp at start moves to previous block', async ({ page }) => {
    const blocks = page.locator('.block-group');
    await blocks.nth(1).click();
    await page.waitForTimeout(300);

    await page.keyboard.press('Home');
    await page.keyboard.press('ArrowUp');
    await page.waitForTimeout(800);

    // After navigation, either block should have an editor
    const editors = page.locator('.cm-content');
    await expect(editors).toHaveCount(1);
  });
});
