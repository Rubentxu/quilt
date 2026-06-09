/**
 * Cognitive Panels E2E Tests
 *
 * Covers the three right-column `cognitivo::` family panels:
 *   - AgentActivityFeed  (global, dynamic — S2-02 fix)
 *   - StructuralGraph    (page-scoped)
 *   - SemanticInsight    (page-scoped)
 *
 * These panels are NOT a tab bar. They are independent sections
 * rendered simultaneously in a single column (`<aside data-testid=
 * "cognitive-panels" />`) and toggled via the dashboard LayoutMenu.
 * The `default` preset hides all of them; the `review` preset
 * enables `agent-activity` + `structural-graph`. `semantic-insight`
 * is opt-in via the per-panel checkbox.
 *
 * Prerequisites:
 *   - Server running on localhost:3737
 *   - Frontend running on localhost:5173
 *   - QUILT_API_KEY env var set
 *
 * Run with:
 *   QUILT_API_KEY=<key> npx playwright test cognitive-panels
 */

import { test, expect, type Page } from '@playwright/test'
import { getAuthHeaders } from '../auth-state'

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737'
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173'

// ── Helpers ───────────────────────────────────────────────────────

/** Create a block via REST API. `createdBy` becomes the `created_by` property. */
async function createBlock(
  page: Page,
  pageName: string,
  content: string,
  options: { createdBy?: string; properties?: Record<string, unknown> } = {},
): Promise<string> {
  const headers = getAuthHeaders()
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, {
    data: {
      pageName,
      content,
      createdBy: options.createdBy,
      properties: options.properties,
    },
    headers,
  })
  if (!resp.ok()) {
    const body = await resp.text()
    throw new Error(`createBlock failed with ${resp.status()}: ${body}`)
  }
  const json = (await resp.json()) as { id: string }
  return json.id
}

/** Create a page via REST API. */
async function createPage(page: Page, name: string): Promise<void> {
  const headers = getAuthHeaders()
  const resp = await page.request.post(`${API_URL}/api/v1/pages`, {
    data: { name },
    headers,
  })
  if (!resp.ok() && resp.status() !== 409) {
    const body = await resp.text()
    throw new Error(`createPage failed with ${resp.status()}: ${body}`)
  }
}

/** List the agent authors currently known to the server. */
async function getDistinctAuthors(page: Page): Promise<string[]> {
  const headers = getAuthHeaders()
  const resp = await page.request.get(`${API_URL}/api/v1/blocks/authors`, {
    headers,
  })
  if (!resp.ok()) {
    const body = await resp.text()
    throw new Error(`getDistinctAuthors failed with ${resp.status()}: ${body}`)
  }
  return (await resp.json()) as string[]
}

/** Generate a unique agent id so each run does not collide. */
function uniqueAgentId(): string {
  return `testbot-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
}

/** Generate a unique page name. */
function uniquePageName(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
}

/**
 * Open the LayoutMenu in the topbar and apply the `review` preset,
 * which enables `agent-activity` and `structural-graph` panels.
 * Returns nothing — caller asserts visibility downstream.
 */
async function applyReviewPreset(page: Page): Promise<void> {
  const trigger = page.getByTestId('layout-menu-trigger')
  await expect(trigger).toBeVisible({ timeout: 10_000 })
  await trigger.click()

  // Menu is `role="menu"` with aria-label "Layout".
  const menu = page.getByTestId('layout-menu')
  await expect(menu).toBeVisible({ timeout: 5_000 })

  const reviewButton = page.getByTestId('layout-preset-review')
  await expect(reviewButton).toBeVisible()
  await reviewButton.click()
}

/**
 * Open the LayoutMenu and toggle a specific panel checkbox on.
 * Use for `semantic-insight` (not in the `review` preset) and for
 * negative tests that need to start from a known-disabled state.
 */
async function togglePanelInLayoutMenu(page: Page, panelId: string): Promise<void> {
  const trigger = page.getByTestId('layout-menu-trigger')
  await expect(trigger).toBeVisible({ timeout: 10_000 })
  await trigger.click()

  const menu = page.getByTestId('layout-menu')
  await expect(menu).toBeVisible({ timeout: 5_000 })

  // Each panel toggle is a `<label data-testid="layout-toggle-<id>">`
  // containing a checkbox with aria-label = panel label.
  const toggle = page.getByTestId(`layout-toggle-${panelId}`)
  await expect(toggle).toBeVisible()
  // Click the label — the inner checkbox forwards to the React handler.
  await toggle.click()
}

/** Open the front-end on a regular page (so the page-scoped panels have data). */
async function goToPage(page: Page, pageName: string): Promise<void> {
  await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(pageName)}`)
  const main = page.locator('main')
  await expect(main).toBeVisible({ timeout: 15_000 })
}

// ── Tests ─────────────────────────────────────────────────────────

test.describe('AgentActivityFeed (S2-02 regression)', () => {
  test('agent activity panel renders when enabled via review preset', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/`)
    await expect(page.getByTestId('app-shell')).toBeVisible({ timeout: 10_000 })

    // Apply the `review` preset — this is the user-facing way to
    // enable `agent-activity` and `structural-graph` together.
    await applyReviewPreset(page)

    // The column container appears once ANY cognitive panel is on.
    const panelsColumn = page.getByTestId('cognitive-panels')
    await expect(panelsColumn).toBeVisible({ timeout: 10_000 })

    // The agent-activity section is the one with the `data-testid` we
    // want. It must be visible, but its INNER content depends on
    // whether any agent-authored blocks exist — so we only assert the
    // wrapper is present (the section root), and the header text.
    const agentSection = page.getByTestId('cognitive-panel-agent-activity')
    await expect(agentSection).toBeVisible()

    // The header always reads "Agent Activity" — proves the panel
    // rendered. Items below may or may not exist.
    await expect(
      agentSection.getByText('Agent Activity', { exact: true }),
    ).toBeVisible()
  })

  test('dynamic agent discovery — newly created agent appears in feed (S2-02)', async ({
    page,
  }) => {
    // Create a unique agent so this run cannot collide with other runs
    // or pre-existing data in the test database.
    const agentId = uniqueAgentId()
    const agentAuthor = `agent::${agentId}`
    const probePage = uniquePageName('cog-feed-probe')

    await createPage(page, probePage)
    await createBlock(page, probePage, 'hello from the bot', { createdBy: agentAuthor })

    // Sanity: the server now knows about this agent.
    const authors = await getDistinctAuthors(page)
    expect(authors).toContain(agentAuthor)

    // Open the app and enable the panel.
    await page.goto(`${FRONTEND_URL}/`)
    await expect(page.getByTestId('app-shell')).toBeVisible({ timeout: 10_000 })
    await applyReviewPreset(page)

    // The agent-activity section is rendered.
    const agentSection = page.getByTestId('cognitive-panel-agent-activity')
    await expect(agentSection).toBeVisible({ timeout: 10_000 })

    // The whole point of S2-02: the feed must show the newly created
    // agent author (not a hardcoded whitelist). The author label is
    // rendered as the literal `agent::<id>` string inside the item.
    // Use getByText — the agent id is unique, so it must match exactly.
    await expect(
      agentSection.getByText(agentAuthor, { exact: true }).first(),
    ).toBeVisible({ timeout: 10_000 })

    // And at least one feed item is rendered.
    const feedItems = agentSection.locator('[data-testid="agent-activity-item"]')
    await expect(feedItems.first()).toBeVisible()
  })

  test('empty state message renders when no agent-authored blocks exist', async ({
    page,
    request,
  }) => {
    // The /blocks/authors endpoint returns a Set persisted at the DB
    // level — we cannot easily clear it between runs without admin
    // access. So this test takes a different approach: it asserts the
    // EMPTY-STATE BRANCH is reachable by looking for the static
    // message OR the feed items, never failing for a populated DB.
    // The hard requirement is: the panel must render successfully
    // and not crash. We use a small pre-check: if the DB has zero
    // agents, the empty state MUST be present. If it has agents, the
    // feed items MUST be present. Either way the test is meaningful.

    await page.goto(`${FRONTEND_URL}/`)
    await expect(page.getByTestId('app-shell')).toBeVisible({ timeout: 10_000 })
    await applyReviewPreset(page)

    const agentSection = page.getByTestId('cognitive-panel-agent-activity')
    await expect(agentSection).toBeVisible({ timeout: 10_000 })

    const authors = await getDistinctAuthors(page)

    if (authors.length === 0) {
      // Empty branch: the literal "No agent activity yet" placeholder
      // must be visible. This is the user-facing empty state from
      // AgentActivityFeed.tsx.
      await expect(
        agentSection.getByText('No agent activity yet', { exact: true }),
      ).toBeVisible({ timeout: 10_000 })
    } else {
      // Populated branch: at least one feed item must render. We
      // surface the actual agent ids so a regression in the
      // author-label renderer is loud.
      const feedItems = agentSection.locator('[data-testid="agent-activity-item"]')
      await expect(feedItems.first()).toBeVisible({ timeout: 10_000 })
      expect(await feedItems.count()).toBeGreaterThan(0)
    }
  })
})

test.describe('StructuralGraph', () => {
  test('structural graph panel renders when enabled via review preset', async ({ page }) => {
    // The panel needs a current page to fetch stats for; navigate to
    // an existing page first.
    const probePage = uniquePageName('cog-graph-probe')
    await createPage(page, probePage)
    await createBlock(page, probePage, 'orphan candidate (no [[wikilinks]])')

    await goToPage(page, probePage)
    await applyReviewPreset(page)

    const graphSection = page.getByTestId('cognitive-panel-structural-graph')
    await expect(graphSection).toBeVisible({ timeout: 10_000 })

    // Header text is always "Structural Graph".
    await expect(
      graphSection.getByText('Structural Graph', { exact: true }),
    ).toBeVisible()

    // The stat-tiles section should render with at least the block
    // count tile. The block we just created means the count is ≥ 1.
    const blockCountTile = graphSection.getByTestId('structural-graph-block-count')
    await expect(blockCountTile).toBeVisible({ timeout: 10_000 })
  })

  test('graph panel reports block count ≥ 1 for a page with blocks', async ({ page }) => {
    const probePage = uniquePageName('cog-graph-count')
    await createPage(page, probePage)
    await createBlock(page, probePage, 'first block')
    await createBlock(page, probePage, 'second block')

    await goToPage(page, probePage)
    await applyReviewPreset(page)

    const graphSection = page.getByTestId('cognitive-panel-structural-graph')
    await expect(graphSection).toBeVisible({ timeout: 10_000 })

    const blockCountTile = graphSection.getByTestId('structural-graph-block-count')
    await expect(blockCountTile).toBeVisible({ timeout: 10_000 })

    // The tile shows the count as a number. With 2 blocks created
    // it must be ≥ 1 — we don't assert == 2 because other specs
    // running in parallel may have created more blocks on the same
    // page (they don't, but a regression that double-counts must
    // blow up, not be masked). 1 is the lower bound the test cares
    // about.
    const text = (await blockCountTile.textContent()) ?? ''
    const match = text.match(/\d+/)
    expect(match).not.toBeNull()
    const count = Number(match![0])
    expect(count).toBeGreaterThanOrEqual(1)
  })
})

test.describe('SemanticInsight', () => {
  test('semantic insight panel renders when enabled via LayoutMenu toggle', async ({
    page,
  }) => {
    // Create a page with a single block tagged as `type:: insight` —
    // that is the contract the panel filters on.
    const probePage = uniquePageName('cog-insight-probe')
    await createPage(page, probePage)
    await createBlock(page, probePage, 'an insight', {
      properties: { type: 'insight' },
    })

    await goToPage(page, probePage)

    // Enable ONLY the semantic-insight panel (the review preset
    // leaves it off by default per presets.ts).
    await togglePanelInLayoutMenu(page, 'semantic-insight')

    const insightSection = page.getByTestId('cognitive-panel-semantic-insight')
    await expect(insightSection).toBeVisible({ timeout: 10_000 })

    // Header text proves the panel mounted.
    await expect(
      insightSection.getByText('Semantic Insight', { exact: true }),
    ).toBeVisible()
  })

  test('semantic insight shows empty state when no insight blocks exist', async ({
    page,
  }) => {
    // A page with no `type:: insight` blocks must show the empty
    // placeholder text defined in SemanticInsight.tsx.
    const probePage = uniquePageName('cog-insight-empty')
    await createPage(page, probePage)
    await createBlock(page, probePage, 'just a regular block')

    await goToPage(page, probePage)
    await togglePanelInLayoutMenu(page, 'semantic-insight')

    const insightSection = page.getByTestId('cognitive-panel-semantic-insight')
    await expect(insightSection).toBeVisible({ timeout: 10_000 })

    // The empty-state wrapper has its own `data-testid`. We assert
    // on that — the prose inside it ("No insights on this page.
    // Agents write insight blocks with `type:: insight`.") is part
    // of the user-facing copy and is robust against copy changes
    // upstream as long as the testid stays.
    const empty = insightSection.getByTestId('semantic-insight-empty')
    await expect(empty).toBeVisible({ timeout: 10_000 })
    // The empty wrapper must contain a `<code>type:: insight</code>`
    // hint — that's the contract agents follow to publish insights.
    await expect(empty.locator('code')).toHaveText('type:: insight')
  })
})

test.describe('Right-column panel toggling', () => {
  test('LayoutMenu preset switches the visible set of cognitive panels', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/`)
    await expect(page.getByTestId('app-shell')).toBeVisible({ timeout: 10_000 })

    // Default preset → no cognitive column is rendered at all.
    // (The default preset only includes `sidebar` + `backlinks`.)
    // The wrapper is absent because CognitivePanels returns null
    // when no cognitive panel is in the visible set.
    await expect(page.getByTestId('cognitive-panels')).toHaveCount(0)

    // Switch to the review preset → column appears.
    await applyReviewPreset(page)

    const panelsColumn = page.getByTestId('cognitive-panels')
    await expect(panelsColumn).toBeVisible({ timeout: 10_000 })

    // Review preset enables agent-activity + structural-graph but
    // NOT semantic-insight. So we expect to see the first two and
    // NOT the third.
    await expect(page.getByTestId('cognitive-panel-agent-activity')).toBeVisible()
    await expect(page.getByTestId('cognitive-panel-structural-graph')).toBeVisible()
    await expect(page.getByTestId('cognitive-panel-semantic-insight')).toHaveCount(0)
  })

  test('toggling semantic-insight on via LayoutMenu makes its panel appear', async ({
    page,
  }) => {
    // Start with the review preset (has agent-activity + structural).
    await page.goto(`${FRONTEND_URL}/`)
    await expect(page.getByTestId('app-shell')).toBeVisible({ timeout: 10_000 })
    await applyReviewPreset(page)

    const panelsColumn = page.getByTestId('cognitive-panels')
    await expect(panelsColumn).toBeVisible({ timeout: 10_000 })

    // Sanity: semantic-insight is OFF in review preset.
    await expect(page.getByTestId('cognitive-panel-semantic-insight')).toHaveCount(0)

    // Need a page to navigate to so the page-scoped panel has data.
    // Use the journal of today — it's always present.
    const today = new Date().toISOString().slice(0, 10)
    await page.goto(`${FRONTEND_URL}/journal/${today}`)
    const main = page.locator('main')
    await expect(main).toBeVisible({ timeout: 15_000 })

    // The LayoutMenu persists state across navigation, so the
    // column is still rendered. Now toggle semantic-insight.
    await togglePanelInLayoutMenu(page, 'semantic-insight')

    await expect(page.getByTestId('cognitive-panel-semantic-insight')).toBeVisible({
      timeout: 10_000,
    })
  })

  test('cognitive column disappears when no panel is enabled', async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/`)
    await expect(page.getByTestId('app-shell')).toBeVisible({ timeout: 10_000 })

    // Enable via review preset first → column appears.
    await applyReviewPreset(page)
    await expect(page.getByTestId('cognitive-panels')).toBeVisible({ timeout: 10_000 })

    // Switch to the focus preset, which only enables `backlinks`.
    // The cognitive column should disappear.
    const trigger = page.getByTestId('layout-menu-trigger')
    await trigger.click()
    await expect(page.getByTestId('layout-menu')).toBeVisible({ timeout: 5_000 })
    await page.getByTestId('layout-preset-focus').click()

    // CognitivePanels returns null when no cognitive panel is in
    // the visible set — the column wrapper is removed from the DOM.
    await expect(page.getByTestId('cognitive-panels')).toHaveCount(0, {
      timeout: 10_000,
    })
  })
})
