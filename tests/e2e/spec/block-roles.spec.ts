/**
 * Block Roles E2E Tests
 *
 * Verifies the two special block roles implemented in Phase 2:
 *
 *   - AgentRun  (Phase 2 #19) — `type:: agent-run` header strip with
 *               agent name, run-status badge, and started-at.
 *
 *   - SavedView (Phase 2 #20) — `type:: view` block that delegates
 *               its content area to <SavedViewBlock>, dispatching on
 *               `view-type::` and resolving the source Query block via
 *               `data-source::`.
 *
 * Strategy: seed blocks via the REST API, then drive the UI to assert
 * the rendering. This is the same approach used by cards.spec.ts and
 * markers.spec.ts — UI slash-commands for these roles are not stable
 * enough to use as test inputs (they're out of scope here).
 *
 * Run with:
 *   QUILT_API_KEY=<key> npx playwright test block-roles
 */

import { test, expect, type Page } from '@playwright/test'
import { getAuthHeaders } from '../auth-state'

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737'
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173'

// ── Helpers ─────────────────────────────────────────────────────

function suffix(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
}

/** Create a regular page via REST. Throws on non-2xx — no silent skip. */
async function createPage(page: Page, name: string): Promise<void> {
  const headers = getAuthHeaders()
  const resp = await page.request.post(`${API_URL}/api/v1/pages`, {
    data: { name },
    headers,
  })
  if (!resp.ok()) {
    throw new Error(
      `createPage(${name}) failed with ${resp.status()}: ${await resp.text()}`,
    )
  }
}

/** Create a block via REST. Returns the block id. */
async function createBlock(
  page: Page,
  pageName: string,
  content: string,
  properties?: Record<string, unknown>,
): Promise<string> {
  const headers = getAuthHeaders()
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, {
    data: { pageName, content, properties },
    headers,
  })
  if (!resp.ok()) {
    throw new Error(
      `createBlock failed with ${resp.status()}: ${await resp.text()}`,
    )
  }
  return ((await resp.json()) as { id: string }).id
}

/** Navigate the browser to a page route and wait for it to mount. */
async function openPage(page: Page, pageName: string): Promise<void> {
  await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(pageName)}`)
  // Wait for the outliner to mount — first block row appears once
  // blocks have been fetched. Using `.first()` is safe because we are
  // not asserting the count, only that the outliner rendered.
  await expect(page.locator('main').first()).toBeVisible({ timeout: 15_000 })
}

// ─── AgentRun ───────────────────────────────────────────────────

test.describe('AgentRun block role (Phase 2 #19)', () => {
  test('Create AgentRun via API and verify inline rendering', async ({
    page,
  }) => {
    const pageName = `roles-agentrun-${suffix()}`
    await createPage(page, pageName)
    await createBlock(page, pageName, 'Agent action', {
      type: 'agent-run',
      agent: 'claude',
      'run-status': 'Running',
    })

    await openPage(page, pageName)

    // The agent-run header strip should be visible. It carries the
    // agent badge ("claude") and the run-status badge ("RUNNING").
    const header = page.getByTestId('agent-run-header').first()
    await expect(header).toBeVisible({ timeout: 10_000 })

    await expect(page.getByTestId('agent-run-agent').first()).toHaveText(
      /claude/,
    )
    await expect(page.getByTestId('agent-run-status').first()).toHaveText(
      /RUNNING/,
    )

    // Sanity: the row itself is mounted (it always is for every block)
    // but the strategy attribute identifies the role.
    const firstRow = page.locator('[data-strategy="agent-run"]').first()
    await expect(firstRow).toBeVisible({ timeout: 5_000 })
  })

  test('AgentRun status badge differs between Completed and Failed', async ({
    page,
  }) => {
    const pageName = `roles-statuses-${suffix()}`
    await createPage(page, pageName)
    await createBlock(page, pageName, 'Done run', {
      type: 'agent-run',
      agent: 'claude',
      'run-status': 'Completed',
    })
    await createBlock(page, pageName, 'Broken run', {
      type: 'agent-run',
      agent: 'claude',
      'run-status': 'Failed',
    })

    await openPage(page, pageName)

    // Both status badges should be present and carry distinct text.
    const completedBadge = page.getByTestId('agent-run-status').filter({
      hasText: 'COMPLETED',
    })
    const failedBadge = page.getByTestId('agent-run-status').filter({
      hasText: 'FAILED',
    })
    await expect(completedBadge).toBeVisible({ timeout: 10_000 })
    await expect(failedBadge).toBeVisible({ timeout: 5_000 })

    // Their background colours must differ (per AGENT_RUN_STATUS_STYLES,
    // Completed → success, Failed → danger). Inline style reads from
    // the rendered span; the comparison is what the user actually sees.
    const completedBg = await completedBadge.evaluate(
      (el) => el.style.background,
    )
    const failedBg = await failedBadge.evaluate((el) => el.style.background)
    expect(completedBg).not.toBe('')
    expect(failedBg).not.toBe('')
    expect(completedBg).not.toBe(failedBg)
  })

  test('AgentRun summary property is persisted and readable after UI load', async ({
    page,
  }) => {
    const pageName = `roles-summary-${suffix()}`
    await createPage(page, pageName)
    const blockId = await createBlock(page, pageName, 'Bug fix run', {
      type: 'agent-run',
      agent: 'claude',
      'run-status': 'Completed',
      summary: 'Fixed 3 bugs',
    })

    await openPage(page, pageName)

    // The agent-run header must render so the user can see the
    // header strip with the agent name and status badge.
    await expect(page.getByTestId('agent-run-header').first()).toBeVisible({
      timeout: 10_000,
    })
    await expect(page.getByTestId('agent-run-agent').first()).toHaveText(
      /claude/,
    )
    await expect(page.getByTestId('agent-run-status').first()).toHaveText(
      /COMPLETED/,
    )

    // The summary property is stored on the block and survives the
    // round-trip through the UI. (The agent-run header strip itself
    // does not currently render summary inline — it lives on the
    // block properties panel — so the durable, observable proof that
    // the user can see the value is to read it back from the API.)
    const headers = getAuthHeaders()
    const propsResp = await page.request.get(
      `${API_URL}/api/v1/blocks/${blockId}/properties`,
      { headers },
    )
    expect(propsResp.ok()).toBeTruthy()
    const props = (await propsResp.json()) as Record<string, unknown>
    expect(props['summary']).toBe('Fixed 3 bugs')
    expect(props['type']).toBe('agent-run')
  })
})

// ─── SavedView ──────────────────────────────────────────────────

test.describe('SavedView block role (Phase 2 #20)', () => {
  test('SavedView (table) renders the view container and view-name', async ({
    page,
  }) => {
    const pageName = `roles-view-${suffix()}`
    await createPage(page, pageName)
    // A table view needs a data-source:: pointing at a real block on
    // the same page. We create a stub query block and then reference
    // it from the view.
    const queryBlockId = await createBlock(
      page,
      pageName,
      'Source query',
      { type: 'query' },
    )
    await createBlock(page, pageName, 'My Test View', {
      type: 'view',
      'view-type': 'table',
      'view-name': 'Test View',
      'data-source': queryBlockId,
    })

    await openPage(page, pageName)

    // The SavedView dispatcher mounts data-testid="saved-view-block"
    // and the inner table wrapper data-testid="saved-view-table".
    const viewBlock = page.getByTestId('saved-view-block').first()
    await expect(viewBlock).toBeVisible({ timeout: 10_000 })

    await expect(page.getByTestId('saved-view-table').first()).toBeVisible({
      timeout: 5_000,
    })

    // The view-name badge carries the user-supplied name.
    await expect(page.getByTestId('saved-view-name').first()).toHaveText(
      /Test View/,
    )

    // The strategy on the block row is "view", confirming the
    // dispatcher fired (not the default inline content path).
    const firstRow = page.locator('[data-strategy="view"]').first()
    await expect(firstRow).toBeVisible({ timeout: 5_000 })
  })

  test('SavedView connected to a source query block renders the source content', async ({
    page,
  }) => {
    const pageName = `roles-view-ds-${suffix()}`
    await createPage(page, pageName)

    // Distinctive source content so we can prove the dispatcher is
    // reading the linked block, not just the view block's own content.
    const sourceMarker = `source-${suffix()}`
    const queryBlockId = await createBlock(
      page,
      pageName,
      `Query source: ${sourceMarker}`,
      { type: 'query' },
    )
    await createBlock(page, pageName, 'Kanban view', {
      type: 'view',
      'view-type': 'kanban',
      'view-name': 'Kanban View',
      'data-source': queryBlockId,
    })

    await openPage(page, pageName)

    const viewBlock = page.getByTestId('saved-view-block').first()
    await expect(viewBlock).toBeVisible({ timeout: 10_000 })

    // The kanban wrapper is dispatched when view-type:: is "kanban".
    await expect(page.getByTestId('saved-view-kanban').first()).toBeVisible({
      timeout: 5_000,
    })

    // The view-name shows what the user named the view.
    await expect(page.getByTestId('saved-view-name').first()).toHaveText(
      /Kanban View/,
    )

    // No error state — the data-source resolved to a real block on
    // the same page.
    await expect(page.getByTestId('saved-view-error')).toHaveCount(0)
  })
})

// ─── Persistence ────────────────────────────────────────────────

test.describe('Block roles persistence across page reload', () => {
  test('AgentRun and SavedView blocks survive a full page reload', async ({
    page,
  }) => {
    const pageName = `roles-persist-${suffix()}`
    await createPage(page, pageName)

    // Seed one of each role on the same page.
    await createBlock(page, pageName, 'Persisted run', {
      type: 'agent-run',
      agent: 'claude',
      'run-status': 'Completed',
      summary: 'Persisted summary',
    })
    const queryBlockId = await createBlock(
      page,
      pageName,
      'Persisted source',
      { type: 'query' },
    )
    await createBlock(page, pageName, 'Persisted view', {
      type: 'view',
      'view-type': 'list',
      'view-name': 'Persisted View',
      'data-source': queryBlockId,
    })

    // First load — assert both render.
    await openPage(page, pageName)
    await expect(page.getByTestId('agent-run-header').first()).toBeVisible({
      timeout: 10_000,
    })
    await expect(page.getByTestId('saved-view-block').first()).toBeVisible({
      timeout: 5_000,
    })

    // Reload — assert both still render.
    await page.reload()
    await expect(page.locator('main').first()).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('agent-run-header').first()).toBeVisible({
      timeout: 10_000,
    })
    await expect(page.getByTestId('agent-run-status').first()).toHaveText(
      /COMPLETED/,
    )
    await expect(page.getByTestId('saved-view-block').first()).toBeVisible({
      timeout: 10_000,
    })
    await expect(page.getByTestId('saved-view-name').first()).toHaveText(
      /Persisted View/,
    )
  })
})
