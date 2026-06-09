/**
 * Graph Lens E2E Tests
 *
 * Covers the Graph Lens V1 endpoint (`GET /api/v1/graph/lens`) and the
 * Graph Lens V2 quick-access buttons (lens selector with keyboard
 * shortcuts 1-4) on the React Graph page.
 *
 * Backend handler: `crates/quilt-server/src/handlers/graph.rs`
 *   - focus DSL: `block:<uuid>`, `page:<name>`, `property:<key>`, or absent.
 *   - depth: 1..=3 inclusive, default 1. Out-of-range → 400.
 *   - Auth: enforced by the global middleware (401 without Bearer).
 *
 * Frontend: `quilt-ui/src/pages/GraphViewPage.tsx`
 *   - The lens selector is a `role="radiogroup"` of 4 buttons.
 *   - Buttons expose `data-lens` ("all" | "page-context" |
 *     "block-subtree" | "property") and a visible label.
 *   - V2 quick-access shortcut: pressing 1, 2, 3, or 4 should switch
 *     the active lens.
 *
 * Prerequisites:
 *   - Server running on http://localhost:3737
 *   - Frontend running on http://localhost:5173 (Playwright spawns via
 *     `webServer` in playwright.config.ts)
 *   - QUILT_API_KEY env var set
 *
 * Run: QUILT_API_KEY=<key> npx playwright test graph-lens
 */

import { test, expect, type APIRequestContext } from '@playwright/test';
import { getAuthHeaders, requireApiKey } from '../auth-state';

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737';
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173';

// Sanity check — fail fast at file load if the key is missing.
requireApiKey();

// ─── Helpers ────────────────────────────────────────────────────────────────

let pageCounter = 0;
function uniquePageName(prefix: string): string {
  pageCounter += 1;
  return `${prefix}-${Date.now()}-${pageCounter}`;
}

interface CreatedBlock {
  id: string;
  pageName: string;
}

/**
 * Create a page via the REST API. The server surfaces a duplicate name
 * as a 500 ("UNIQUE constraint failed") rather than a 409, so we treat
 * that body as a successful no-op.
 */
async function createPage(
  request: APIRequestContext,
  pageName: string,
): Promise<void> {
  const resp = await request.post(`${API_URL}/api/v1/pages`, {
    headers: getAuthHeaders(),
    data: { name: pageName, journal: false, journalDay: null },
  });
  if (!resp.ok()) {
    const body = await resp.text();
    if (!body.includes('UNIQUE constraint failed: pages.name')) {
      throw new Error(`createPage failed (${resp.status()}): ${body}`);
    }
  }
}

/** Create a root block on an EXISTING page and return its id. */
async function createRootBlock(
  request: APIRequestContext,
  pageName: string,
  content: string,
): Promise<CreatedBlock> {
  // The /blocks endpoint requires the page to exist beforehand.
  await createPage(request, pageName);
  const resp = await request.post(`${API_URL}/api/v1/blocks`, {
    headers: getAuthHeaders(),
    data: { pageName, content },
  });
  if (!resp.ok()) {
    throw new Error(
      `createRootBlock failed (${resp.status()}): ${await resp.text()}`,
    );
  }
  const json = (await resp.json()) as { id: string };
  return { id: json.id, pageName };
}

/** Create a child block under `parentId` on an existing page. */
async function createChildBlock(
  request: APIRequestContext,
  pageName: string,
  parentId: string,
  content: string,
): Promise<{ id: string }> {
  const resp = await request.post(`${API_URL}/api/v1/blocks`, {
    headers: getAuthHeaders(),
    data: { pageName, content, parentId },
  });
  if (!resp.ok()) {
    throw new Error(
      `createChildBlock failed (${resp.status()}): ${await resp.text()}`,
    );
  }
  return (await resp.json()) as { id: string };
}

/** Clean up all blocks on a page (best-effort; ignores errors). */
async function cleanupPage(request: APIRequestContext, pageName: string) {
  const headers = getAuthHeaders();
  const resp = await request.get(
    `${API_URL}/api/v1/pages/${encodeURIComponent(pageName)}/blocks`,
    { headers },
  );
  if (!resp.ok()) return;
  const blocks = (await resp.json()) as Array<{ id: string }>;
  for (const b of blocks) {
    await request.delete(`${API_URL}/api/v1/blocks/${b.id}`, { headers });
  }
}

// ─── Test group: Graph page navigation ──────────────────────────────────────

test.describe('Graph Page Navigation', () => {
  test('@smoke clicking Graph in sidebar loads the graph page', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/`);
    await expect(page.getByTestId('app-shell')).toBeVisible({ timeout: 10_000 });

    // The sidebar nav item is rendered as a <Link> with the visible
    // text "Vista de Grafo" (the project uses Rioplatense copy in
    // the sidebar). Match by the link's accessible name — the visible
    // text inside the <span> child of the link.
    await page.getByRole('link', { name: /vista de grafo/i }).first().click();
    await page.waitForURL(/\/graph/, { timeout: 10_000 });

    // The graph page must render its header and the lens selector.
    await expect(
      page.getByRole('heading', { name: /knowledge graph/i }),
    ).toBeVisible({ timeout: 10_000 });

    await expect(page.getByTestId('lens-selector')).toBeVisible({
      timeout: 10_000,
    });
  });

  test('graph page exposes zoom and lens controls', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/graph`);

    // Lens selector (V1/V2 quick-access row) — accessible as a
    // radiogroup labelled "Graph lens".
    const lensGroup = page.getByRole('radiogroup', { name: /graph lens/i });
    await expect(lensGroup).toBeVisible({ timeout: 10_000 });

    // All four lens buttons must be present.
    for (const label of ['All', 'Page context', 'Block subtree', 'Property filter']) {
      await expect(
        page.getByRole('radio', { name: new RegExp(`^${label}$`, 'i') }),
      ).toBeVisible();
    }

    // Zoom controls (Lucide icon-only buttons carry a `title` attribute).
    await expect(page.getByRole('button', { name: /zoom in/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /zoom out/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /reset view/i })).toBeVisible();
  });
});

// ─── Test group: Graph Lens API ─────────────────────────────────────────────

test.describe('Graph Lens API', () => {
  test('GET /api/v1/graph/lens returns 200 with a valid focus', async ({ request }) => {
    // Create a real block so the focus is valid.
    const pageName = uniquePageName('lens-api-ok');
    const root = await createRootBlock(
      request,
      pageName,
      `Lens API ok @ ${Date.now()}`,
    );

    try {
      const resp = await request.get(`${API_URL}/api/v1/graph/lens`, {
        headers: getAuthHeaders(),
        params: { focus: `block:${root.id}`, depth: '1' },
      });
      expect(resp.status()).toBe(200);
      const data = await resp.json();
      expect(data.nodes).toBeDefined();
      expect(Array.isArray(data.nodes)).toBe(true);
      expect(data.edges).toBeDefined();
      expect(Array.isArray(data.edges)).toBe(true);
    } finally {
      await cleanupPage(request, pageName);
    }
  });

  test('GET /api/v1/graph/lens with depth=0 returns 400 (below MIN_DEPTH)', async ({ request }) => {
    const resp = await request.get(`${API_URL}/api/v1/graph/lens`, {
      headers: getAuthHeaders(),
      params: { focus: 'page:any', depth: '0' },
    });
    expect(resp.status()).toBe(400);
  });

  test('GET /api/v1/graph/lens with depth=5 returns 400 (above MAX_DEPTH)', async ({ request }) => {
    // Validates the S2-05 fix: depth bounds are named constants
    // MIN_DEPTH=1 and MAX_DEPTH=3 with docs.
    const resp = await request.get(`${API_URL}/api/v1/graph/lens`, {
      headers: getAuthHeaders(),
      params: { focus: 'page:any', depth: '5' },
    });
    expect(resp.status()).toBe(400);
  });

  test('GET /api/v1/graph/lens without auth returns 401', async ({ request }) => {
    // Empty headers — the global middleware must reject.
    const resp = await request.get(`${API_URL}/api/v1/graph/lens`, {
      params: { focus: 'page:any', depth: '1' },
    });
    expect(resp.status()).toBe(401);
  });
});

// ─── Test group: Lens API returns a real subgraph ───────────────────────────

test.describe('Graph Lens API — Subgraph Data', () => {
  test('lens with a focus block returns related blocks from the parent-child tree', async ({ request }) => {
    // Build a small tree: root → c1 → gc1 (3 levels deep).
    const pageName = uniquePageName('lens-tree');
    const root = await createRootBlock(
      request,
      pageName,
      `Lens root @ ${Date.now()}`,
    );

    try {
      const c1 = await createChildBlock(
        request,
        pageName,
        root.id,
        'Lens child 1',
      );
      const c2 = await createChildBlock(
        request,
        pageName,
        root.id,
        'Lens child 2',
      );
      const gc1 = await createChildBlock(
        request,
        pageName,
        c1.id,
        'Lens grandchild 1',
      );

      // depth=1 → just the focus block.
      const r1 = await request.get(`${API_URL}/api/v1/graph/lens`, {
        headers: getAuthHeaders(),
        params: { focus: `block:${root.id}`, depth: '1' },
      });
      expect(r1.status()).toBe(200);
      const d1 = await r1.json();
      const d1Ids = (d1.nodes as Array<{ id: string }>).map(n => n.id);
      expect(d1Ids).toContain(root.id);
      expect(d1Ids).not.toContain(c1.id);
      expect(d1Ids).not.toContain(c2.id);
      expect(d1Ids).not.toContain(gc1.id);

      // depth=2 → root + both children.
      const r2 = await request.get(`${API_URL}/api/v1/graph/lens`, {
        headers: getAuthHeaders(),
        params: { focus: `block:${root.id}`, depth: '2' },
      });
      expect(r2.status()).toBe(200);
      const d2 = await r2.json();
      const d2Ids = (d2.nodes as Array<{ id: string }>).map(n => n.id);
      expect(d2Ids).toContain(root.id);
      expect(d2Ids).toContain(c1.id);
      expect(d2Ids).toContain(c2.id);
      expect(d2Ids).not.toContain(gc1.id);

      // depth=3 → root + both children + the grandchild.
      const r3 = await request.get(`${API_URL}/api/v1/graph/lens`, {
        headers: getAuthHeaders(),
        params: { focus: `block:${root.id}`, depth: '3' },
      });
      expect(r3.status()).toBe(200);
      const d3 = await r3.json();
      const d3Ids = (d3.nodes as Array<{ id: string }>).map(n => n.id);
      expect(d3Ids).toContain(root.id);
      expect(d3Ids).toContain(c1.id);
      expect(d3Ids).toContain(c2.id);
      expect(d3Ids).toContain(gc1.id);
    } finally {
      await cleanupPage(request, pageName);
    }
  });
});

// ─── Test group: Lens buttons (V2) ──────────────────────────────────────────

test.describe('Lens Buttons (V2)', () => {
  test('lens selector is visible on the graph page', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/graph`);

    const lensGroup = page.getByRole('radiogroup', { name: /graph lens/i });
    await expect(lensGroup).toBeVisible({ timeout: 10_000 });

    // The four lens buttons render as radios inside the group.
    for (const label of ['All', 'Page context', 'Block subtree', 'Property filter']) {
      await expect(
        page.getByRole('radio', { name: new RegExp(`^${label}$`, 'i') }),
      ).toBeVisible();
    }
  });

  test('clicking a non-default lens button updates the active state', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/graph`);
    await expect(
      page.getByRole('radiogroup', { name: /graph lens/i }),
    ).toBeVisible({ timeout: 10_000 });

    // The "All" button is the default-active state.
    const allButton = page.getByRole('radio', { name: /^all$/i });
    const pageContextButton = page.getByRole('radio', {
      name: /^page context$/i,
    });
    await expect(allButton).toHaveAttribute('data-active', 'true');

    // Click the "Page context" lens.
    await pageContextButton.click();

    // The "Page context" button should now be active and "All" should not.
    await expect(pageContextButton).toHaveAttribute('data-active', 'true');
    await expect(allButton).toHaveAttribute('data-active', 'false');
  });

  test('keyboard shortcut "3" switches to the Block subtree lens', async ({ page }) => {
    // V2 quick-access: pressing 1, 2, 3, or 4 anywhere on the page sets
    // the active lens. This test exercises key "3" → Block subtree.
    //
    // The shortcut must be implemented at the page level; the test will
    // fail (per the project rules: tests MUST FAIL, never skip) until
    // the keyboard handler is added to GraphViewPage.

    await page.goto(`${FRONTEND_URL}/graph`);
    await expect(
      page.getByRole('radiogroup', { name: /graph lens/i }),
    ).toBeVisible({ timeout: 10_000 });

    // Move focus to the document body so the key event has somewhere to
    // land that is not an input element.
    await page.evaluate(() => {
      if (document.activeElement instanceof HTMLElement) {
        document.activeElement.blur();
      }
    });

    await page.keyboard.press('3');

    const blockSubtree = page.getByRole('radio', { name: /^block subtree$/i });
    await expect(blockSubtree).toHaveAttribute('data-active', 'true', {
      timeout: 5_000,
    });
  });
});
