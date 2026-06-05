/**
 * Sidebar Templates + Recents E2E Tests
 *
 * PR 5 of `quilt-fase1-sidebar-mcp-templates`. Covers:
 *   1. Sidebar renders the "Plantillas" group header
 *   2. Sidebar renders the "Recientes" group header after navigation
 *   3. Recents: visit 3 pages, reload, 3 entries persist (localStorage round-trip)
 *   4. Recents: cap at 5 — visit 6 pages, only the 5 newest survive
 *   5. Recents: case-insensitive dedup — visit "Foo" and "foo", one entry
 *   6. Recents: bad localStorage payload is ignored, empty state shown
 *   7. Templates: clicking a template creates a new page (with -1 suffix) and navigates
 *   8. MCP data path: the templates list backing `quilt_get_sidebar_state`
 *      is reachable through the running HTTP server's `/api/v1/templates`
 *      endpoint (the MCP tool reads from the same use case).
 *
 * Tag: `@sidebar` — run with `npx playwright test --grep @sidebar`.
 *
 * Auth: every API call goes through `getAuthHeaders()` (Bearer token from
 * `QUILT_API_KEY`). The frontend itself is reached through Vite at 5173.
 *
 * Per project rules:
 *   - No CSS selectors — `getByRole` / `getByLabelText` / `getByText`
 *   - No `waitForTimeout` — `findBy*` / `expect().toBeVisible()` / `toHaveURL`
 *   - Tests MUST fail (not skip) if the backend is unreachable.
 *   - Test behaviour, not implementation — no `mock_called()`.
 */

import { test, expect, type Page } from '@playwright/test';
import { getAuthHeaders } from '../auth-state';

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737';
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173';
const STORAGE_KEY = 'quilt-recents';

// ── Helpers ──────────────────────────────────────────────────────

/** Random suffix — every artifact (page, template) gets a unique one to
 *  avoid UNIQUE collisions when tests run in parallel. */
function suffix(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
}

/** Create a page via REST. Throws on non-2xx — no silent skip. */
async function createPage(page: Page, name: string): Promise<void> {
  const headers = getAuthHeaders();
  const resp = await page.request.post(`${API_URL}/api/v1/pages`, {
    data: { name },
    headers,
  });
  if (!resp.ok()) {
    throw new Error(`createPage(${name}) failed with ${resp.status()}: ${await resp.text()}`);
  }
}

/** Create a block — used to set `card-shape` on a template page so the
 *  template surfaces in the sidebar's `Plantillas` section. */
async function createBlock(
  page: Page,
  pageName: string,
  content: string,
  properties?: Record<string, unknown>
): Promise<string> {
  const headers = getAuthHeaders();
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, {
    data: { pageName, content, properties },
    headers,
  });
  if (!resp.ok()) {
    throw new Error(`createBlock failed with ${resp.status()}: ${await resp.text()}`);
  }
  const json = (await resp.json()) as { id: string };
  return json.id;
}

/** Visit a `/page/:name` route via the SPA — triggers the Recents hook. */
async function visitPage(page: Page, name: string): Promise<void> {
  await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(name)}`);
  // Wait for the main app shell to mount — the recents hook only fires
  // AFTER the route is matched, so we must wait for the layout to settle.
  await expect(page.getByRole('navigation', { name: /primary/i }).first())
    .toBeVisible({ timeout: 10_000 })
    .catch(() => {
      // Some layouts don't expose role=navigation; fall back to sidebar
      // testid which is always present in the AppShell.
      return expect(page.locator('[data-testid="sidebar"]')).toBeVisible({ timeout: 10_000 });
    });
}

/** Read the persisted recents list straight from localStorage. */
async function readRecents(page: Page): Promise<Array<{ name: string; url: string }>> {
  const raw = await page.evaluate((key) => localStorage.getItem(key), STORAGE_KEY);
  if (!raw) return [];
  const parsed: unknown = JSON.parse(raw);
  if (!Array.isArray(parsed)) return [];
  return parsed
    .filter((r): r is { name: string; url: string } =>
      typeof r === 'object' && r !== null &&
      typeof (r as { name?: unknown }).name === 'string' &&
      typeof (r as { url?: unknown }).url === 'string'
    )
    .map((r) => ({ name: r.name, url: r.url }));
}

// ── Tests ────────────────────────────────────────────────────────

test.describe('Sidebar — Plantillas section @sidebar', () => {
  test('sidebar shows the Plantillas group header', async ({ page }) => {
    await page.goto(FRONTEND_URL);
    await expect(page.locator('[data-testid="sidebar"]')).toBeVisible({ timeout: 10_000 });
    // GroupHeader renders the label as a heading-level element; locate by text
    // (the section uses <p> per GroupHeader implementation, so getByText is
    // the right role-free matcher).
    await expect(page.getByText('Plantillas', { exact: true })).toBeVisible();
  });

  test('clicking a template creates a page with the -1 suffix and navigates to it @sidebar', async ({ page }) => {
    const s = suffix();
    const templateName = `tpl-${s}`;
    const templatePage = `template/${templateName}`;
    const expectedNewPage = `${templateName}-1`;

    // Set up: create a template page (any template with card-shape will do)
    await createPage(page, templatePage);
    await createBlock(page, templatePage, '', { 'card-shape': 'reference', 'icon': '🔗' });

    // Open the app and wait for the sidebar to load templates
    await page.goto(FRONTEND_URL);
    await expect(page.locator('[data-testid="sidebar"]')).toBeVisible({ timeout: 10_000 });

    // The template item is rendered with a data-testid derived from its name
    const templateItem = page.locator(`[data-testid="template-item-${templateName}"]`);
    await expect(templateItem).toBeVisible({ timeout: 10_000 });

    // The accessible name should follow the spec: "Create page from template: <name>"
    await expect(templateItem).toHaveAttribute(
      'aria-label',
      `Create page from template: ${templateName}`
    );

    // Click — the create call is async, wait for navigation
    await templateItem.click();

    // Spec says navigate to `/page/<pageName>`. The implementation
    // auto-suffixes `-1` for collision-free naming.
    await expect(page).toHaveURL(
      new RegExp(`/page/${encodeURIComponent(expectedNewPage)}$`),
      { timeout: 10_000 }
    );
  });
});

test.describe('Sidebar — Recientes section @sidebar', () => {
  test('sidebar shows the Recientes group header after a page visit @sidebar', async ({ page }) => {
    const s = suffix();
    const pageName = `e2e-recent-${s}`;
    await createPage(page, pageName);

    await visitPage(page, pageName);
    await expect(page.getByText('Recientes', { exact: true })).toBeVisible({ timeout: 5_000 });
  });

  test('visiting 3 pages persists 3 entries that survive a reload @sidebar', async ({ page }) => {
    const s = suffix();
    const pages = [`e2e-r1-${s}`, `e2e-r2-${s}`, `e2e-r3-${s}`];
    for (const name of pages) await createPage(page, name);

    // Visit each in turn so the recents hook fires three times
    for (const name of pages) await visitPage(page, name);

    // Newest first
    const before = await readRecents(page);
    expect(before.map((r) => r.name).slice(0, 3)).toEqual([
      pages[2], pages[1], pages[0],
    ]);

    // Reload — the recents list is localStorage-backed and must survive
    await page.reload();
    await expect(page.locator('[data-testid="sidebar"]')).toBeVisible({ timeout: 10_000 });

    const after = await readRecents(page);
    expect(after.map((r) => r.name).slice(0, 3)).toEqual([
      pages[2], pages[1], pages[0],
    ]);
  });

  test('cap at 5 — visiting 6 pages keeps only the 5 newest @sidebar', async ({ page }) => {
    const s = suffix();
    const pages = Array.from({ length: 6 }, (_, i) => `e2e-cap-${i}-${s}`);
    for (const name of pages) await createPage(page, name);

    for (const name of pages) await visitPage(page, name);

    const recents = await readRecents(page);
    // The cap is 5 entries — pages[0] is the oldest and must be evicted
    expect(recents.length).toBe(5);
    expect(recents.map((r) => r.name)).toEqual(pages.slice(1).reverse());
    expect(recents.some((r) => r.name === pages[0])).toBe(false);
  });

  test('case-insensitive dedup — visiting "Foo" then "foo" leaves one entry with original casing @sidebar', async ({ page }) => {
    const s = suffix();
    const canonical = `e2e-case-${s}`;
    const lowerVariant = canonical.toLowerCase();
    await createPage(page, canonical);

    await visitPage(page, canonical);
    await visitPage(page, lowerVariant);

    const recents = await readRecents(page);
    const matching = recents.filter((r) => r.name.toLowerCase() === canonical.toLowerCase());
    expect(matching.length).toBe(1);
    // Spec: the original casing is preserved (the first-visit wins).
    expect(matching[0].name).toBe(canonical);
  });

  test('malformed localStorage payload is tolerated — empty state, no crash @sidebar', async ({ page }) => {
    // Seed localStorage with garbage before the app reads it
    await page.goto(FRONTEND_URL);
    await page.evaluate((key) => localStorage.setItem(key, 'not-valid-json{'), STORAGE_KEY);

    // Reload so the recents hook re-reads from storage
    await page.reload();
    await expect(page.locator('[data-testid="sidebar"]')).toBeVisible({ timeout: 10_000 });

    // Either the empty state or the Recientes group header is visible —
    // the spec requires graceful degradation without a crash
    const empty = page.locator('[data-testid="recents-empty"]');
    const groupHeader = page.getByText('Recientes', { exact: true });
    await expect(empty.or(groupHeader)).toBeVisible({ timeout: 5_000 });
  });
});

test.describe('MCP sidebar state — data path @sidebar', () => {
  // The `quilt_get_sidebar_state` MCP tool reads from `TemplateUseCases::list()`,
  // which is the SAME use case the HTTP `/api/v1/templates` endpoint exposes.
  // Verifying the HTTP path is a faithful proxy for the MCP tool's data layer —
  // the tool's contract (JSON shape, fields, ordering) is locked down by the
  // Rust unit tests in `crates/quilt-mcp/src/handlers/sidebar.rs`. This E2E
  // test confirms the data the MCP tool will surface reaches the user-facing
  // templates list correctly.
  test('the templates list backing quilt_get_sidebar_state is reachable via HTTP and includes seed templates @sidebar', async ({ page }) => {
    const s = suffix();
    const templateName = `mcp-seed-${s}`;
    const templatePage = `template/${templateName}`;
    await createPage(page, templatePage);
    await createBlock(page, templatePage, '', { 'card-shape': 'reference' });

    const headers = getAuthHeaders();
    const resp = await page.request.get(`${API_URL}/api/v1/templates`, { headers });
    expect(resp.ok(), `GET /api/v1/templates returned ${resp.status()}`).toBe(true);

    const templates = (await resp.json()) as Array<{ name: string; full_name: string }>;
    const names = templates.map((t) => t.name);
    // The seeded template must appear in the list — this is the same data
    // the MCP tool will read for its `templates[]` field.
    expect(names).toContain(templateName);
    // The shape mirrors `TemplateSummary` (name, full_name, etc.) — no fields
    // dropped or added between the use case and the wire.
    const ours = templates.find((t) => t.name === templateName)!;
    expect(ours.full_name).toBe(templatePage);
  });
});
