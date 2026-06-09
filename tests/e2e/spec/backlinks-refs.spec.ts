/**
 * Backlinks & References E2E Tests
 *
 * Covers Q028 (Editable Backlinks) and Q029 (Unlinked Ref Queue), plus
 * the underlying reference linking primitives that feed both:
 *
 *   - GET /api/v1/pages/:name/backlinks  (the panel reads from this)
 *   - PUT /api/v1/references/:blockId     (Q028: edit backlink context)
 *   - The frontend `[[Page]]` and `((uuid))` link renderers
 *   - The localStorage-backed Unlinked Ref Queue surfaced in the panel
 *
 * Auth: every API call uses `getAuthHeaders()` (Bearer token from
 * QUILT_API_KEY). The panel itself fetches through the same client.
 *
 * Conventions used in this file:
 *   - `getByRole` / `getByLabelText` / `getByText` for user-facing locators
 *   - `data-testid` only on panel internals that the components expose
 *     for testing (mirrors the in-app BacklinksPanel / UnlinkedRefQueue
 *     test suites — not a CSS selector workaround)
 *   - NO `waitForTimeout` — every wait is a Playwright auto-retry assertion
 *   - Tests MUST FAIL, never skip
 *
 * Run with:
 *   QUILT_API_KEY=<key> npx playwright test backlinks-refs
 */

import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import { getAuthHeaders } from '../auth-state';

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737';
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173';

// ─── Block id format helpers ──────────────────────────────────────
//
// The Quilt API has two ways to identify a block:
//
//   - `id` is a UUID string with hyphens (e.g. "d2e8d278-0ae5-4d52-97ef-0b32a5bf23f0").
//     This is what `POST /api/v1/blocks` returns and what every other
//     endpoint uses for path parameters (e.g. `PUT /api/v1/blocks/:id`).
//
//   - `blockId` from `GET /api/v1/blocks/search` is the SAME id but
//     hex-encoded (uppercase, no hyphens). The FTS5 query uses
//     `hex(b.id)` for the column projection.
//
// The UnlinkedRefQueue hook (Q029) takes the search-result `blockId`
// verbatim and renders it as `data-block-id` on every row. So when
// we want to assert that "the queue contains the block we just
// created", we need the HEX form, not the UUID form.
//
// `blockIdToHex` converts a UUID-shaped string into the hex form
// the search endpoint and the unlinked-ref-queue use.
function blockIdToHex(uuid: string): string {
  return uuid.replace(/-/g, '').toUpperCase();
}

// ─── Unique-naming helpers ─────────────────────────────────────────
//
// Every test creates its own pages so they can run in parallel without
// colliding on backlink / unlinked-queue state. The suffix uses
// `Date.now()` plus a per-test counter so two `test()` calls in the
// same millisecond still get distinct names.

let nameCounter = 0;
function uniquePageName(prefix: string): string {
  nameCounter += 1;
  return `pw-${prefix}-${Date.now()}-${nameCounter}`;
}

function uniqueNonExistentPageName(prefix: string): string {
  // Names that have NEVER been created — the unlinked-ref queue
  // surfaces only refs to pages that don't exist yet.
  return `pw-nonexistent-${prefix}-${Date.now()}-${nameCounter}`;
}

// ─── API helpers ───────────────────────────────────────────────────

interface CreatedPage {
  name: string;
  blockIds: string[];
}

async function createPage(req: APIRequestContext, name: string): Promise<void> {
  const resp = await req.post(`${API_URL}/api/v1/pages`, {
    data: { name },
    headers: getAuthHeaders(),
  });
  if (!resp.ok() && resp.status() !== 409) {
    const body = await resp.text();
    throw new Error(`createPage(${name}) failed with ${resp.status()}: ${body}`);
  }
}

async function createBlock(
  req: APIRequestContext,
  pageName: string,
  content: string,
  parentId: string | null = null,
): Promise<string> {
  const resp = await req.post(`${API_URL}/api/v1/blocks`, {
    data: { pageName, content, parentId },
    headers: getAuthHeaders(),
  });
  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`createBlock failed with ${resp.status()}: ${body}`);
  }
  const json = (await resp.json()) as { id: string };
  return json.id;
}

async function getBlocks(
  req: APIRequestContext,
  pageName: string,
): Promise<Array<{ id: string; content: string }>> {
  const resp = await req.get(
    `${API_URL}/api/v1/pages/${encodeURIComponent(pageName)}/blocks`,
    { headers: getAuthHeaders() },
  );
  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`getBlocks failed with ${resp.status()}: ${body}`);
  }
  return (await resp.json()) as Array<{ id: string; content: string }>;
}

interface BacklinkDto {
  sourceBlockId: string;
  sourcePageName: string;
  contentPreview: string;
  context: string;
}

async function getBacklinks(
  req: APIRequestContext,
  pageName: string,
): Promise<BacklinkDto[]> {
  const resp = await req.get(
    `${API_URL}/api/v1/pages/${encodeURIComponent(pageName)}/backlinks`,
    { headers: getAuthHeaders() },
  );
  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`getBacklinks failed with ${resp.status()}: ${body}`);
  }
  return (await resp.json()) as BacklinkDto[];
}

async function setReferenceContext(
  req: APIRequestContext,
  sourceBlockId: string,
  targetPageName: string,
  context: string | null,
): Promise<BacklinkDto> {
  const resp = await req.put(
    `${API_URL}/api/v1/references/${encodeURIComponent(sourceBlockId)}?targetPage=${encodeURIComponent(targetPageName)}`,
    {
      data: { context },
      headers: { ...getAuthHeaders(), 'Content-Type': 'application/json' },
    },
  );
  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`setReferenceContext failed with ${resp.status()}: ${body}`);
  }
  return (await resp.json()) as BacklinkDto;
}

async function deleteAllBlocks(req: APIRequestContext, pageName: string) {
  const blocks = await getBlocks(req, pageName);
  for (const block of blocks) {
    await req.delete(`${API_URL}/api/v1/blocks/${block.id}`, {
      headers: getAuthHeaders(),
    });
  }
}

async function setupTwoMutuallyLinkedPages(
  req: APIRequestContext,
  label: string,
): Promise<{ sourcePage: CreatedPage; targetPage: CreatedPage }> {
  const sourceName = uniquePageName(`source-${label}`);
  const targetName = uniquePageName(`target-${label}`);

  // Order matters: create both pages first, THEN add blocks that
  // reference each other, so the back-end ref index can resolve
  // names to page IDs at the time it parses the content.
  await createPage(req, sourceName);
  await createPage(req, targetName);

  const sourceBlockIds: string[] = [];
  sourceBlockIds.push(
    await createBlock(
      req,
      sourceName,
      `Source points to [[${targetName}]] — g3-ref linking.`,
    ),
  );
  const targetBlockIds: string[] = [];
  targetBlockIds.push(
    await createBlock(
      req,
      targetName,
      `Target points back to [[${sourceName}]] — mutual link.`,
    ),
  );

  return {
    sourcePage: { name: sourceName, blockIds: sourceBlockIds },
    targetPage: { name: targetName, blockIds: targetBlockIds },
  };
}

// ─── UI helpers ────────────────────────────────────────────────────

/**
 * Make sure the right-side BacklinksPanel is open. The AppShell
 * exposes a top-bar toggle whose `aria-label` flips between
 * "Open backlinks panel" and "Close backlinks panel" based on
 * `backlinksOpen` in the PanelVisibilityContext.
 *
 * The panel starts open by default (per the `default` preset in
 * `features/dashboard/presets.ts`) — but a previous test in the
 * same run may have closed it, so we always verify and toggle if
 * needed. The internal content is collapsed on first render and
 * needs the header to be clicked — see `expandBacklinksPanel`.
 */
async function openBacklinksPanel(page: Page) {
  // The panel root is what we actually want visible.
  const panel = page.getByTestId('backlinks-panel');
  // Wait for the panel to mount (it does so as soon as a page
  // route renders AppShell).
  await expect(panel).toBeVisible({ timeout: 10_000 });
}

async function expandBacklinksPanel(page: Page) {
  // The header is a real <button> with `aria-expanded` reflecting the
  // current state. We assert visible first (panel must be open), then
  // click only if not already expanded.
  const header = page.getByTestId('backlinks-panel-header');
  await expect(header).toBeVisible({ timeout: 5_000 });
  const expanded = await header.getAttribute('aria-expanded');
  if (expanded !== 'true') {
    await header.click();
  }
  await expect(header).toHaveAttribute('aria-expanded', 'true');
}

/**
 * Clear the UnlinkedRefQueue by wiping the `localStorage` key the
 * hook reads on mount. We do this at the top of every queue test
 * so leftover state from a previous test (or from a previous run)
 * doesn't pollute the assertions.
 */
async function clearUnlinkedRefQueue(page: Page) {
  await page.evaluate(() => {
    window.localStorage.removeItem('unlinked-ref-queue');
  });
}

// ─── Tests ─────────────────────────────────────────────────────────

test.describe('Backlinks Panel', () => {
  test('shows mutual references when a page is opened', async ({ page, request }) => {
    const { sourcePage, targetPage } = await setupTwoMutuallyLinkedPages(
      request,
      'mutual',
    );

    try {
      // Navigate to source page; the Backlinks panel should now show
      // the block on `targetPage` that points back to it.
      await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(sourcePage.name)}`);

      // The right-side backlinks panel exists; open it.
      await openBacklinksPanel(page);
      await expandBacklinksPanel(page);

      // The panel renders a header labeled "Linked References" with a
      // count. The count must be at least 1 because `targetPage` links
      // to `sourcePage`.
      const header = page.getByTestId('backlinks-panel-header');
      await expect(header).toBeVisible();
      await expect(header).toContainText(/linked references/i);

      const countBadge = page.getByTestId('backlinks-panel-count');
      await expect(countBadge).toBeVisible();
      // Count text is just a number — assert via API, then via UI.
      const apiBacklinks = await getBacklinks(request, sourcePage.name);
      expect(apiBacklinks.length).toBeGreaterThan(0);

      // The panel should mention the source page name (targetPage links
      // back to sourcePage, so the backlink's group header is targetPage).
      // Each backlink row lives under a group header that displays the
      // source page name. We use a flexible regex to match the name
      // inside a long contentPreview that the panel may show.
      await expect(
        page.getByText(new RegExp(escapeRegExp(targetPage.name))).first(),
      ).toBeVisible({ timeout: 10_000 });
    } finally {
      await deleteAllBlocks(request, sourcePage.name);
      await deleteAllBlocks(request, targetPage.name);
    }
  });

  test('shows incoming link from a block that references the page', async ({
    page,
    request,
  }) => {
    // Same direction as the previous test (page A has a block that
    // links to page B, so page B's backlinks list shows the block
    // on A) but with a single-direction link. The panel is fed by
    // wiki-link refs only — plain prose mentions go to the
    // UnlinkedRefQueue instead (Q029).
    const a = uniquePageName('linker');
    const b = uniquePageName('linkee');
    await createPage(request, a);
    await createPage(request, b);
    const linkerBlockId = await createBlock(
      request,
      a,
      `Plain prose, but also links to [[${b}]] here.`,
    );

    try {
      // Open page B — its backlinks should include the block on A.
      await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(b)}`);
      await openBacklinksPanel(page);
      await expandBacklinksPanel(page);

      // The API must report the backlink before the UI can show it.
      const apiBacklinks = await getBacklinks(request, b);
      expect(apiBacklinks.length).toBeGreaterThan(0);
      const apiHit = apiBacklinks.find((x) => x.sourceBlockId === linkerBlockId);
      expect(apiHit).toBeDefined();

      // The panel must render that specific block id as a row. The
      // component sets `data-testid={`backlinks-item-${sourceBlockId}`}`
      // on every row, so we can target the exact one.
      await expect(
        page.getByTestId(`backlinks-item-${linkerBlockId}`),
      ).toBeVisible({ timeout: 10_000 });
    } finally {
      await deleteAllBlocks(request, a);
      await deleteAllBlocks(request, b);
    }
  });
});

test.describe('Editable Backlinks (Q028)', () => {
  test('PUT /references/:blockId updates the context shown in the panel', async ({
    page,
    request,
  }) => {
    const { sourcePage, targetPage } = await setupTwoMutuallyLinkedPages(
      request,
      'q028-edit',
    );

    // Pre-condition: the source page has exactly one block (the one we
    // just created), and that block is what the target's backlinks list
    // points to. Its current `context` is the default snippet — the
    // server-built preview of the source block's content.
    const before = await getBacklinks(request, targetPage.name);
    expect(before.length).toBeGreaterThan(0);
    const sourceBlockId = before[0].sourceBlockId;
    expect(sourceBlockId).toBe(sourcePage.blockIds[0]);

    // The PUT endpoint accepts a JSON body with `context: string | null`.
    // We send a recognisable string so we can assert it appears in the
    // panel after the update.
    const customContext = `Edited snippet ${Date.now()}`;
    const updated = await setReferenceContext(
      request,
      sourceBlockId,
      targetPage.name,
      customContext,
    );
    expect(updated.context).toBe(customContext);

    try {
      // Open the target page in the UI; the panel should now render
      // the user-edited snippet.
      await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(targetPage.name)}`);
      await openBacklinksPanel(page);
      await expandBacklinksPanel(page);

      // The BacklinksPanel renders the context inside an element
      // tagged `backlinks-item-context-<sourceBlockId>`. The edit
      // button next to it is `backlinks-item-edit-<sourceBlockId>`.
      const contextEl = page.getByTestId(
        `backlinks-item-context-${sourceBlockId}`,
      );
      await expect(contextEl).toBeVisible({ timeout: 10_000 });
      await expect(contextEl).toHaveText(customContext);
    } finally {
      // Clear the override so the page is back to default before
      // cleanup; otherwise the next test that creates a same-named
      // page could see a stale context.
      await setReferenceContext(request, sourceBlockId, targetPage.name, null);
      await deleteAllBlocks(request, sourcePage.name);
      await deleteAllBlocks(request, targetPage.name);
    }
  });

  test('edited backlink context persists across a page reload', async ({
    page,
    request,
  }) => {
    const { sourcePage, targetPage } = await setupTwoMutuallyLinkedPages(
      request,
      'q028-reload',
    );

    const before = await getBacklinks(request, targetPage.name);
    const sourceBlockId = before[0].sourceBlockId;

    const customContext = `Persists-after-reload ${Date.now()}`;
    await setReferenceContext(
      request,
      sourceBlockId,
      targetPage.name,
      customContext,
    );

    try {
      // Visit the page once, confirm the edit shows, then reload and
      // confirm it's still there.
      await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(targetPage.name)}`);
      await openBacklinksPanel(page);
      await expandBacklinksPanel(page);

      const contextEl = page.getByTestId(
        `backlinks-item-context-${sourceBlockId}`,
      );
      await expect(contextEl).toBeVisible({ timeout: 10_000 });
      await expect(contextEl).toHaveText(customContext);

      // Reload — the page state comes from the API, so the override
      // we just wrote must come back.
      await page.reload();
      await openBacklinksPanel(page);
      await expandBacklinksPanel(page);

      const reloadedEl = page.getByTestId(
        `backlinks-item-context-${sourceBlockId}`,
      );
      await expect(reloadedEl).toBeVisible({ timeout: 10_000 });
      await expect(reloadedEl).toHaveText(customContext);
    } finally {
      await setReferenceContext(request, sourceBlockId, targetPage.name, null);
      await deleteAllBlocks(request, sourcePage.name);
      await deleteAllBlocks(request, targetPage.name);
    }
  });
});

test.describe('Unlinked Ref Queue (Q029)', () => {
  test('unlinked reference appears in the queue', async ({ page, request }) => {
    // Set up: a target page that EXISTS, with a source block on a
    // different page that mentions the target's name in plain prose
    // (no [[...]] wrapper). The UnlinkedRefQueue hook scans for those.
    const targetName = uniquePageName('q029-target');
    const sourceName = uniquePageName('q029-source');
    await createPage(request, targetName);
    await createPage(request, sourceName);
    // The plain mention is what triggers the queue. It MUST be a
    // word-bounded match (see `detectMentions` in unlinkedRefQueue.ts).
    const sourceBlockId = await createBlock(
      request,
      sourceName,
      `Some prose that mentions ${targetName} without linking it.`,
    );

    try {
      // The queue is per-origin localStorage, so start from a clean
      // slate for this test.
      await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(targetName)}`);
      await clearUnlinkedRefQueue(page);
      await page.reload();

      // Open the panel and expand the content. The queue lives
      // inside the expanded panel.
      await openBacklinksPanel(page);
      await expandBacklinksPanel(page);

      // The UnlinkedRefQueue section header is `unlinked-ref-queue`.
      // Its badge shows the count. We assert via the testid the
      // component exposes, not via CSS.
      const queue = page.getByTestId('unlinked-ref-queue');
      await expect(queue).toBeVisible({ timeout: 10_000 });

      // The queue is collapsed by default — click the header to open
      // it. The header is a real <button> with `aria-expanded`.
      const queueHeader = page.getByTestId('unlinked-ref-queue-header');
      const headerExpanded = await queueHeader.getAttribute('aria-expanded');
      if (headerExpanded !== 'true') {
        await queueHeader.click();
      }
      await expect(queueHeader).toHaveAttribute('aria-expanded', 'true');

      // At least one candidate must be listed, and it must reference
      // the source block we just created.
      const items = page.getByTestId('unlinked-ref-queue-item');
      await expect(items.first()).toBeVisible({ timeout: 10_000 });

      // The queue renders `data-block-id` as the HEX form of the
      // block's UUID (the search endpoint's projection). The
      // `data-block-id` attribute lives on the same element as the
      // `data-testid="unlinked-ref-queue-item"`, so we filter with a
      // combined CSS attribute selector rather than `.filter({ has })`
      // (which would look for a descendant element).
      const hexId = blockIdToHex(sourceBlockId);
      const ourItem = page.locator(
        `[data-testid="unlinked-ref-queue-item"][data-block-id="${hexId}"]`,
      );
      await expect(ourItem).toHaveCount(1);
    } finally {
      await deleteAllBlocks(request, sourceName);
      await deleteAllBlocks(request, targetName);
    }
  });

  test('clicking Link promotes a plain mention to a [[wiki]] link', async ({
    page,
    request,
  }) => {
    // Same setup as the previous test: a plain mention of an existing
    // target page on a different page.
    const targetName = uniquePageName('q029-link-target');
    const sourceName = uniquePageName('q029-link-source');
    await createPage(request, targetName);
    await createPage(request, sourceName);
    const sourceBlockId = await createBlock(
      request,
      sourceName,
      `Random sentence mentioning ${targetName} in plain text.`,
    );

    try {
      await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(targetName)}`);
      await clearUnlinkedRefQueue(page);
      await page.reload();

      await openBacklinksPanel(page);
      await expandBacklinksPanel(page);

      // Open the unlinked-queue section and find the row for our block.
      const queueHeader = page.getByTestId('unlinked-ref-queue-header');
      if ((await queueHeader.getAttribute('aria-expanded')) !== 'true') {
        await queueHeader.click();
      }
      await expect(queueHeader).toHaveAttribute('aria-expanded', 'true');

      const ourItem = page.locator(
        `[data-testid="unlinked-ref-queue-item"][data-block-id="${blockIdToHex(sourceBlockId)}"]`,
      );
      await expect(ourItem).toHaveCount(1, { timeout: 10_000 });

      // Click the Link button on our row. The hook's `link` action
      // wraps the mention in `[[...]]` and PATCHes the block, then
      // removes the candidate from the queue.
      const linkButton = ourItem.getByTestId('unlinked-ref-queue-link');
      await linkButton.click();

      // The hook's `link` action does an `api.updateBlock` that
      // PATCHes the block content. After that, the queue should
      // eventually drop the candidate (the local state filters it
      // out, and the persisted queue in localStorage is also
      // updated by `removeCandidate`).
      await expect(ourItem).toHaveCount(0, { timeout: 10_000 });

      // Verify via API: the source block's content must now contain
      // the canonical [[Target]] form. Re-fetch the block list.
      const after = await getBlocks(request, sourceName);
      const src = after.find((b) => b.id === sourceBlockId);
      expect(src).toBeDefined();
      // Case-insensitive contains — `targetName` is the canonical form
      // the server stores; the mention may have been typed in a
      // different case but `linkifyMention` always uses the canonical
      // pageName. We accept either.
      expect(src!.content.toLowerCase()).toContain(
        `[[${targetName.toLowerCase()}]]`,
      );
    } finally {
      await deleteAllBlocks(request, sourceName);
      await deleteAllBlocks(request, targetName);
    }
  });

  test('clicking Dismiss removes the candidate from the queue', async ({
    page,
    request,
  }) => {
    const targetName = uniquePageName('q029-dismiss-target');
    const sourceName = uniquePageName('q029-dismiss-source');
    await createPage(request, targetName);
    await createPage(request, sourceName);
    const sourceBlockId = await createBlock(
      request,
      sourceName,
      `Another plain mention of ${targetName} goes here.`,
    );

    try {
      await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(targetName)}`);
      await clearUnlinkedRefQueue(page);
      await page.reload();

      await openBacklinksPanel(page);
      await expandBacklinksPanel(page);

      const queueHeader = page.getByTestId('unlinked-ref-queue-header');
      if ((await queueHeader.getAttribute('aria-expanded')) !== 'true') {
        await queueHeader.click();
      }
      await expect(queueHeader).toHaveAttribute('aria-expanded', 'true');

      const ourItem = page.locator(
        `[data-testid="unlinked-ref-queue-item"][data-block-id="${blockIdToHex(sourceBlockId)}"]`,
      );
      await expect(ourItem).toHaveCount(1, { timeout: 10_000 });

      // Click the Dismiss button on our row. The hook removes the
      // candidate from both React state and localStorage.
      const dismissButton = ourItem.getByTestId('unlinked-ref-queue-dismiss');
      await dismissButton.click();

      await expect(ourItem).toHaveCount(0, { timeout: 10_000 });

      // The persisted queue must NOT contain our block anymore. The
      // stored format uses the HEX form of the block id (same
      // projection the search endpoint uses).
      const persisted = await page.evaluate(() => {
        const raw = window.localStorage.getItem('unlinked-ref-queue');
        if (!raw) return [];
        try {
          return JSON.parse(raw) as Array<{ blockId: string }>;
        } catch {
          return [];
        }
      });
      expect(
        persisted.find((c) => c.blockId === blockIdToHex(sourceBlockId)),
      ).toBeUndefined();
    } finally {
      await deleteAllBlocks(request, sourceName);
      await deleteAllBlocks(request, targetName);
    }
  });
});

test.describe('Reference linking', () => {
  test('[[PageName]] syntax renders as a clickable link in read mode', async ({
    page,
    request,
  }) => {
    // Set up: a target page that exists, and a source page with a
    // block that contains a [[Target]] link.
    const targetName = uniquePageName('reflink-target');
    const sourceName = uniquePageName('reflink-source');
    await createPage(request, targetName);
    await createPage(request, sourceName);
    await createBlock(request, sourceName, `Links to [[${targetName}]] here.`);

    try {
      await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(sourceName)}`);
      // Wait for the read-mode block content to render.
      const readContent = page.locator('.block-content-read').first();
      await expect(readContent).toBeVisible({ timeout: 10_000 });

      // InlineContent renders [[PageName]] as an <a> element with the
      // page name as the link text. Find it by accessible name (the
      // visible text inside the <a>).
      const link = page.getByRole('link', { name: targetName }).first();
      await expect(link).toBeVisible({ timeout: 10_000 });
      // The href is the canonical /page/<name> route.
      await expect(link).toHaveAttribute(
        'href',
        `/page/${encodeURIComponent(targetName)}`,
      );
    } finally {
      await deleteAllBlocks(request, sourceName);
      await deleteAllBlocks(request, targetName);
    }
  });

  test('((block-uuid)) syntax renders as a block reference', async ({
    page,
    request,
  }) => {
    // Set up: a target page with at least one block whose UUID we
    // can reference from a different block on the same page.
    const pageName = uniquePageName('blockref');
    await createPage(request, pageName);
    const targetBlockId = await createBlock(
      request,
      pageName,
      'I am the referenced block.',
    );
    await createBlock(
      request,
      pageName,
      `See ((${targetBlockId})) for context.`,
    );

    try {
      await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(pageName)}`);
      const readContent = page.locator('.block-content-read').first();
      await expect(readContent).toBeVisible({ timeout: 10_000 });

      // InlineContent renders block refs as <span> elements with a
      // `title="Block ref: <id>"` and a small Hash icon. The visible
      // text is the preview of the referenced block's content.
      const blockRef = page.locator(`[title="Block ref: ${targetBlockId}"]`);
      await expect(blockRef).toBeVisible({ timeout: 10_000 });
      // The preview text comes from the referenced block's content.
      // The renderer caps at 80 chars and appends an ellipsis if it
      // overflows; either form is acceptable.
      const preview = await blockRef.textContent();
      expect(preview).toBeTruthy();
      expect(preview!.replace(/…$/, '')).toContain('I am the referenced block');
    } finally {
      await deleteAllBlocks(request, pageName);
    }
  });

  test('clicking a [[Page]] link navigates to that page', async ({
    page,
    request,
  }) => {
    const targetName = uniquePageName('nav-target');
    const sourceName = uniquePageName('nav-source');
    await createPage(request, targetName);
    await createPage(request, sourceName);
    await createBlock(request, sourceName, `Click me: [[${targetName}]]`);

    try {
      await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(sourceName)}`);
      const readContent = page.locator('.block-content-read').first();
      await expect(readContent).toBeVisible({ timeout: 10_000 });

      // Click the wikilink. The handler calls `e.stopPropagation()`
      // and then `navigate({ to: '/page/$name', ... })`.
      const link = page.getByRole('link', { name: targetName }).first();
      await expect(link).toBeVisible({ timeout: 10_000 });
      await link.click();

      // Navigation lands on /page/<target>. The AppShell's
      // `data-testid="breadcrumb"` is a reliable marker for "we are
      // on a page route" and shows the current page name.
      await expect(page).toHaveURL(
        new RegExp(`/page/${escapeRegExp(targetName)}$`),
        { timeout: 10_000 },
      );

      const breadcrumb = page.getByTestId('breadcrumb');
      await expect(breadcrumb).toBeVisible({ timeout: 5_000 });
      await expect(breadcrumb).toHaveText(targetName);
    } finally {
      await deleteAllBlocks(request, sourceName);
      await deleteAllBlocks(request, targetName);
    }
  });
});

// ─── Utilities ─────────────────────────────────────────────────────

/**
 * Escape a string for safe inclusion in a `RegExp` constructor. The
 * page names we generate don't currently contain regex metachars,
 * but if a future test parameterises this with user input we don't
 * want it to silently match more than expected.
 */
function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
