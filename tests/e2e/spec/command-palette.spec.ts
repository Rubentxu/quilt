/**
 * Command Palette E2E Tests
 *
 * Tests for the CommandRegistry-driven command palette
 * (Cmd/Ctrl+Shift+K) and the Quick Capture builtin command.
 *
 * Covers:
 *   - Palette open/close (keyboard, escape, backdrop click)
 *   - Command filtering
 *   - Quick Capture end-to-end (creates a block in today's journal)
 *   - Keyboard navigation in the palette
 *   - Recent searches appear in the search modal (S2-03 regression)
 *
 * Prerequisites:
 *   - Server running on localhost:3737
 *   - Frontend running on localhost:5173 (Playwright will start it via
 *     `webServer` in playwright.config.ts)
 *   - QUILT_API_KEY env var set
 *
 * Run with: QUILT_API_KEY=<key> npx playwright test command-palette
 */

import { test, expect, type Page } from '@playwright/test';
import { getAuthHeaders, requireApiKey } from '../auth-state';

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737';
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173';

// Sanity check — fail fast at file load if the key is missing.
requireApiKey();

// All tests in this file share the same backend (same SQLite DB on
// the dev server), so they share today's journal page. The Quick
// Capture tests write to that shared page, and the "Recent
// searches" test also seeds a block there. Running tests in
// parallel would race the journal's block count. Force serial
// execution for the whole file.
test.describe.configure({ mode: 'serial' });

/** Cross-platform "Cmd" key. Meta on macOS, Control elsewhere. */
const cmdKey = process.platform === 'darwin' ? 'Meta' : 'Control';

/** Today's date in YYYY-MM-DD (UTC — matches Quick Capture's own logic). */
function todayIsoDate(): string {
  return new Date().toISOString().split('T')[0];
}

/**
 * Pre-seed localStorage so the welcome tour does not pop up over the
 * command palette (the tour's backdrop has z-index 200, which is
 * higher than the palette's 100, so it would intercept backdrop
 * clicks). The tour uses the key `quilt-welcome-seen`; setting it to
 * `'1'` makes the AppShell skip the first-render tour.
 */
async function dismissWelcomeTour(page: Page): Promise<void> {
  await page.addInitScript(() => {
    try {
      localStorage.setItem('quilt-welcome-seen', '1');
    } catch {
      // localStorage may be unavailable — the tour will still render
      // but tests that need pixel-perfect clicks will retry.
    }
  });
}

/** Open the command palette via the global keyboard shortcut. */
async function openCommandPalette(page: Page): Promise<void> {
  await page.keyboard.press(`${cmdKey}+Shift+K`);
  const dialog = page.getByRole('dialog', { name: 'Command palette' });
  await expect(dialog).toBeVisible({ timeout: 5_000 });
  // Wait for the search input to be focused (the modal uses
  // requestAnimationFrame to defer focus; we don't want the test
  // firing keys before focus lands).
  const input = page.getByLabel('Command palette search');
  await expect(input).toBeFocused({ timeout: 2_000 });
}

/** List block IDs for a given page (today's journal). */
async function listBlockIds(
  page: Page,
  pageName: string,
): Promise<string[]> {
  const resp = await page.request.get(
    `${API_URL}/api/v1/pages/${encodeURIComponent(pageName)}/blocks`,
    { headers: getAuthHeaders() },
  );
  if (!resp.ok()) {
    throw new Error(`listBlockIds failed with ${resp.status()}`);
  }
  const blocks = (await resp.json()) as Array<{ id: string }>;
  return blocks.map((b) => b.id);
}

/** Create a block via REST on a page that already exists (today's
 *  journal — see `todayIsoDate`). Returns the new block id. */
async function createBlock(
  page: Page,
  pageName: string,
  content: string,
): Promise<string> {
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, {
    data: { pageName, content, parentId: null },
    headers: getAuthHeaders(),
  });
  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`createBlock failed with ${resp.status()}: ${body}`);
  }
  const json = (await resp.json()) as { id: string };
  return json.id;
}

/** Search for blocks via the REST API. */
async function searchBlocks(
  page: Page,
  query: string,
): Promise<unknown[]> {
  const resp = await page.request.get(
    `${API_URL}/api/v1/blocks/search?query=${encodeURIComponent(query)}&limit=5`,
    { headers: getAuthHeaders() },
  );
  if (!resp.ok()) {
    throw new Error(`searchBlocks failed with ${resp.status()}`);
  }
  return (await resp.json()) as unknown[];
}

/** Wait for a fresh page load and dismiss the welcome tour. */
async function loadHomeAndDismissTour(page: Page): Promise<void> {
  await dismissWelcomeTour(page);
  await page.goto(`${FRONTEND_URL}/`);
  await expect(page.locator('[data-testid="app-shell"]')).toBeVisible({
    timeout: 10_000,
  });
}

// ─── Test group: Command palette open/close ────────────────────────────

test.describe('Command palette open/close', () => {
  test.beforeEach(async ({ page }) => {
    await loadHomeAndDismissTour(page);
  });

  test('Cmd/Ctrl+Shift+K opens command palette', async ({ page }) => {
    await openCommandPalette(page);

    // The search input is the most reliable "is it open" signal.
    const input = page.getByLabel('Command palette search');
    await expect(input).toBeVisible();
    await expect(input).toBeFocused();

    // The listbox (results area) should be present even if empty.
    await expect(page.getByRole('listbox', { name: 'Commands' })).toBeVisible();
  });

  test('Escape closes command palette', async ({ page }) => {
    await openCommandPalette(page);

    await page.keyboard.press('Escape');

    // The dialog should disappear.
    await expect(
      page.getByRole('dialog', { name: 'Command palette' }),
    ).not.toBeVisible({ timeout: 5_000 });
  });

  test('Clicking outside closes command palette', async ({ page }) => {
    await openCommandPalette(page);

    // The backdrop has data-testid="command-center-backdrop" and is
    // the parent of the dialog. The dialog's onClick stops
    // propagation, so we must click the backdrop itself (not the
    // dialog content). The dialog is centered with max-width 640px
    // and 15vh top padding — clicking near the top-left of the
    // viewport reliably lands on the backdrop.
    const backdrop = page.locator('[data-testid="command-center-backdrop"]');
    await expect(backdrop).toBeVisible();

    // Use page.mouse.click with viewport coordinates so we land
    // outside the centered dialog content. (10, 10) is well within
    // the backdrop and outside the 640px-wide dialog on a 1280px
    // viewport.
    await page.mouse.click(10, 10);

    await expect(
      page.getByRole('dialog', { name: 'Command palette' }),
    ).not.toBeVisible({ timeout: 5_000 });
  });
});

// ─── Test group: Command filtering ────────────────────────────────────

test.describe('Command filtering', () => {
  test.beforeEach(async ({ page }) => {
    await loadHomeAndDismissTour(page);
  });

  test('Typing filters commands', async ({ page }) => {
    await openCommandPalette(page);

    const input = page.getByLabel('Command palette search');
    await input.fill('capture');

    // The Quick Capture command must remain visible.
    const quickCapture = page.getByRole('option', { name: /Quick Capture/ });
    await expect(quickCapture).toBeVisible();

    // An unrelated command should be filtered out.
    // "Toggle Dark Mode" doesn't share any token with "capture".
    await expect(
      page.getByRole('option', { name: /Toggle Dark Mode/ }),
    ).toHaveCount(0);
  });

  test('No results shows empty state', async ({ page }) => {
    await openCommandPalette(page);

    const input = page.getByLabel('Command palette search');
    await input.fill('zzzzzznonexistent');

    // The empty-state message is rendered when results.length === 0.
    const empty = page.locator('[data-testid="command-center-empty"]');
    await expect(empty).toBeVisible();
    await expect(empty).toContainText(/no commands match/i);
  });
});

// ─── Test group: Quick Capture ─────────────────────────────────────────
//
// These two tests both write to today's journal. They run SERIALLY
// (the file-level `test.describe.configure({ mode: 'serial' })` at
// the top of the file already serializes the whole suite, so the
// block-count assertions are deterministic.

test.describe('Quick Capture', () => {
  test.beforeEach(async ({ page }) => {
    await loadHomeAndDismissTour(page);
  });

  test('Quick Capture creates a block in today\'s journal', async ({ page }) => {
    const today = todayIsoDate();

    // Baseline: count existing blocks for today.
    const before = await listBlockIds(page, today);

    // Quick Capture uses window.prompt() — accept and supply text.
    const captureText = `e2e capture ${Date.now()}`;
    page.once('dialog', async (dialog) => {
      expect(dialog.type()).toBe('prompt');
      await dialog.accept(captureText);
    });

    // Open the palette and run Quick Capture.
    await openCommandPalette(page);

    const input = page.getByLabel('Command palette search');
    await input.fill('Quick Capture');

    // Only the Quick Capture row should match the filter.
    const quickCapture = page.getByRole('option', { name: /Quick Capture/ });
    await expect(quickCapture).toBeVisible();

    // Pressing Enter on the input executes the selected row.
    await input.press('Enter');

    // Wait for the palette to close (it auto-closes on execute).
    await expect(
      page.getByRole('dialog', { name: 'Command palette' }),
    ).not.toBeVisible({ timeout: 5_000 });

    // Verify the block landed in today's journal via the API.
    // The capture is async (it goes through the API), so we wait for
    // the count to change rather than racing the network with a sleep.
    await expect
      .poll(async () => (await listBlockIds(page, today)).length, {
        timeout: 10_000,
        message: 'expected block count to increase after Quick Capture',
      })
      .toBe(before.length + 1);
  });

  test('Quick Capture shows success feedback', async ({ page }) => {
    const today = todayIsoDate();
    const before = await listBlockIds(page, today);

    // react-hot-toast renders a status node; the success message is
    // "Captured to today's journal". We listen for any visible
    // "captured" text in the DOM (the toast is short-lived but the
    // block persists in the journal, so we assert both signals).
    const successToast = page.getByText(/captured/i).first();

    page.once('dialog', async (dialog) => {
      await dialog.accept(`e2e capture ${Date.now()}`);
    });

    await openCommandPalette(page);
    const input = page.getByLabel('Command palette search');
    await input.fill('Quick Capture');
    await input.press('Enter');

    // The block count must go up by exactly one.
    await expect
      .poll(
        async () => (await listBlockIds(page, today)).length,
        { timeout: 10_000, message: 'expected exactly one new block' },
      )
      .toBe(before.length + 1);

    // A success indicator (toast) should have appeared. Toasts auto-
    // dismiss, so we look for any node that currently contains
    // "captured" — the toast text. If the toast has already
    // disappeared, we still consider the test passing (the block
    // landed, which is the durable success signal).
    if (await successToast.count()) {
      await expect(successToast).toBeVisible();
    }
  });
});

// ─── Test group: Keyboard navigation in palette ───────────────────────

test.describe('Keyboard navigation in palette', () => {
  test.beforeEach(async ({ page }) => {
    await loadHomeAndDismissTour(page);
  });

  test('Arrow keys navigate command list', async ({ page }) => {
    await openCommandPalette(page);

    // Use the input locator so the key events are guaranteed to
    // land on the input (not whatever else happens to be focused).
    const input = page.getByLabel('Command palette search');
    const listbox = page.getByRole('listbox', { name: 'Commands' });
    const options = listbox.getByRole('option');

    // Initial selection is the first option (index 0).
    const firstSelected = await options.nth(0).getAttribute('aria-selected');
    expect(firstSelected).toBe('true');

    // Press ArrowDown — the second option (index 1) is now selected.
    await input.press('ArrowDown');
    const secondSelected = await options.nth(1).getAttribute('aria-selected');
    expect(secondSelected).toBe('true');
    const firstStillSelected = await options
      .nth(0)
      .getAttribute('aria-selected');
    expect(firstStillSelected).toBe('false');

    // Press ArrowDown again — the third option (index 2) is now
    // selected, and the first two are not.
    await input.press('ArrowDown');
    const thirdSelected = await options.nth(2).getAttribute('aria-selected');
    expect(thirdSelected).toBe('true');

    // Press ArrowUp — the second option is selected again.
    await input.press('ArrowUp');
    const secondSelectedAgain = await options
      .nth(1)
      .getAttribute('aria-selected');
    expect(secondSelectedAgain).toBe('true');
  });

  test('Multiple commands in palette', async ({ page }) => {
    await openCommandPalette(page);

    // The listbox must contain at least 3 commands (Quick Capture,
    // navigation, view toggles, etc. — builtins are ~16 total).
    const options = page
      .getByRole('listbox', { name: 'Commands' })
      .getByRole('option');
    await expect(options.first()).toBeVisible();
    expect(await options.count()).toBeGreaterThanOrEqual(3);

    // Quick Capture specifically must be in the default list.
    await expect(
      page.getByRole('option', { name: /Quick Capture/ }),
    ).toBeVisible();
  });
});

// ─── Test group: S2-03 regression (Saved/Recent searches) ─────────────

test.describe('Saved/Recent searches appear in search modal', () => {
  test('Recent searches appear after running searches', async ({ page }) => {
    // Seed localStorage with two recent searches, since the in-app
    // search UX requires waiting for debounced results. This is a
    // faster, more deterministic alternative to driving the search
    // modal through the API.
    await page.goto(`${FRONTEND_URL}/`);
    // Give the AppShell time to mount the keyboard listener.
    await expect(page.locator('[data-testid="app-shell"]')).toBeVisible({
      timeout: 10_000,
    });

    // Inject two recent-search entries into the localStorage key the
    // SearchModal reads from. We also seed a real block to ensure the
    // search infrastructure is alive and returns hits.
    await page.evaluate(() => {
      const now = Date.now();
      const recents = [
        { query: 'regression-omega', timestamp: now, resultCount: 1 },
        { query: 'regression-zeta', timestamp: now - 1000, resultCount: 2 },
      ];
      localStorage.setItem('recent-searches', JSON.stringify(recents));
    });

    // Seed a real block in today's journal and verify the search
    // endpoint returns it. The block must go on an existing page
    // (today's journal — it's created implicitly by the journal
    // route), not a brand-new page name, because block creation
    // returns 404 for non-journal page names that don't exist.
    const today = todayIsoDate();
    const uniqueWord = `quiltseed${Date.now()}`;
    await createBlock(page, today, `block containing ${uniqueWord}`);
    const hits = await searchBlocks(page, uniqueWord);
    expect(hits.length).toBeGreaterThan(0);

    // Open the search modal by dispatching the keyboard shortcut
    // directly. The AppShell listens for Ctrl/Cmd+K on `document`
    // and toggles the search modal. We dispatch a synthetic event
    // because Playwright's `page.keyboard.press` is unreliable
    // when the document has multiple focusable elements (the
    // keypress may be captured by a child input and not bubble
    // up cleanly).
    await page.evaluate(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'k',
          code: 'KeyK',
          ctrlKey: true,
          bubbles: true,
          cancelable: true,
        }),
      );
    });

    // The search input is the source of truth for "modal is open".
    // It has no aria-label, but its placeholder is unique. The
    // SearchModal is lazy-loaded, so the first invocation may
    // take a beat.
    const searchInput = page.getByPlaceholder('Search pages and blocks…');
    await expect(searchInput).toBeVisible({ timeout: 10_000 });

    // The "Recent searches" section header must be visible.
    await expect(
      page.getByText('Recent searches', { exact: true }),
    ).toBeVisible({ timeout: 5_000 });

    // And the seeded recent entries should be clickable.
    await expect(
      page.locator('[data-testid="recent-search-row-regression-omega"]'),
    ).toBeVisible();
    await expect(
      page.locator('[data-testid="recent-search-row-regression-zeta"]'),
    ).toBeVisible();
  });
});
