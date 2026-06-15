/**
 * Slash Commands E2E Tests
 *
 * Covers the full slash-command surface registered in
 * `quilt-ui/src/features/outliner-tiptap/slashRegistry.tsx` — 32
 * distinct user-facing commands organised by category:
 *
 *   - **Menu interaction** (3 tests) — opens on `/`, filters as the
 *     user types, closes on Escape.
 *   - **Status / marker** (7) — `/todo`, `/doing`, `/done`, `/now`,
 *     `/later`, `/cancelled`, `/waiting` flip the block's `marker` to the
 *     title-cased `TaskMarker` value. `/doing` is a known gap — the
 *     server's `TaskMarker` enum does not include "Doing", so the
 *     test asserts the contract (marker=null after the call) until
 *     the backend grows the variant.
 *   - **Priority** (3) — `/priority A`, `/priority B`, `/priority C`
 *     write the corresponding `Priority` enum to the block.
 *   - **Dates** (4) — `/today` and `/tomorrow` insert the ISO date
 *     into the block content; `/deadline` and `/scheduled` insert the
 *     `prop:: ` syntax for the property parser to pick up.
 *   - **References** (2) — `/page reference` and `/block embed`
 *     insert `[[` and `((` respectively for the autocomplete.
 *   - **Templates** (1) — `/new from template` fires the wizard
 *     that prompts for a page name and template choice.
 *   - **Comments** (1) — `/add comment` creates a child block with
 *     `type:: comment`.
 *   - **Block types** (11) — `/text`, `/h1`, `/h2`, `/h3`, `/bullet`,
 *     `/numbered`, `/todo`, `/quote`, `/code`, `/divider`, `/image`
 *     flip the block's `blockType` and the value PERSISTS across a
 *     full page reload (regression guard for P0 fix #6 — slash
 *     blockType used to be lost on save).
 *   - **Roles** (3) — `/task`, `/query`, `/card` write structured
 *     `properties` on the block (type:: task / type:: query + dsl:: /
 *     card-shape::). The role menu handlers are property transforms
 *     (NOT blockType changes), so the test asserts via the
 *     `/api/v1/blocks/:id/properties` endpoint, not via blockType.
 *
 * Total: 35 tests. The `/todo` command is exercised TWICE — once as
 * a status setter (marker=Todo) and once as a blockType setter
 * (blockType=todo) — to cover both slash items that share the
 * label `TODO` / `To-do`.
 *
 * Tag: `@slash-commands` — run with `npx playwright test --grep @slash-commands`.
 *
 * Auth: every API call goes through `getAuthHeaders()` (Bearer token from
 * `QUILT_API_KEY`). The frontend itself is reached through Vite at 5173.
 *
 * Per project rules:
 *   - No CSS selectors — `getByRole` / `getByLabelText` / `getByText`
 *   - No `waitForTimeout` — `findBy*` / `expect().toBeVisible()` / `toHaveURL`
 *   - Tests MUST fail (not skip) if the backend is unreachable.
 *   - Test behaviour, not implementation — assert on user-visible state
 *     (DOM text, API record) rather than React internals.
 *
 * Manual execution:
 *   just dev
 *   # in another shell:
 *   QUILT_API_KEY=$(grep VITE_QUILT_API_KEY quilt-ui/.env | cut -d= -f2) \
 *     npx playwright test --grep @slash-commands
 */

import { test, expect, type Page } from '@playwright/test'
import { getAuthHeaders } from '../auth-state'

const API_URL = process.env.API_BASE_URL || 'http://localhost:3737'
const FRONTEND_URL = process.env.BASE_URL || 'http://localhost:5173'

// ── Helpers ──────────────────────────────────────────────────────

/** Random suffix — every artifact (page, block) gets a unique one to
 *  avoid UNIQUE collisions when tests run in parallel. */
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
    throw new Error(`createPage(${name}) failed with ${resp.status()}: ${await resp.text()}`)
  }
}

/** Create a block via REST. Returns the block id. */
async function createBlock(
  page: Page,
  pageName: string,
  content: string,
): Promise<string> {
  const headers = getAuthHeaders()
  const resp = await page.request.post(`${API_URL}/api/v1/blocks`, {
    data: { pageName, content },
    headers,
  })
  if (!resp.ok()) {
    throw new Error(`createBlock failed with ${resp.status()}: ${await resp.text()}`)
  }
  return ((await resp.json()) as { id: string }).id
}

/** Fetch all blocks for a page (REST). */
async function getPageBlocks(
  page: Page,
  pageName: string,
): Promise<Array<{
  id: string
  content: string
  blockType: string
  marker: string | null
  properties: Record<string, unknown>
}>> {
  const headers = getAuthHeaders()
  const resp = await page.request.get(
    `${API_URL}/api/v1/pages/${encodeURIComponent(pageName)}/blocks`,
    { headers },
  )
  if (!resp.ok()) {
    throw new Error(`getPageBlocks failed with ${resp.status()}: ${await resp.text()}`)
  }
  return (await resp.json()) as Array<{
    id: string
    content: string
    blockType: string
    marker: string | null
    properties: Record<string, unknown>
  }>
}

/** Delete every block on a page (best-effort cleanup). */
async function deleteAllBlocks(page: Page, pageName: string): Promise<void> {
  const headers = getAuthHeaders()
  const blocks = await getPageBlocks(page, pageName)
  for (const block of blocks) {
    await page.request.delete(`${API_URL}/api/v1/blocks/${block.id}`, { headers })
  }
}

/** Open a regular page in the browser, wait for the seeded block to
 *  appear, click it to enter edit mode, and return the
 *  `contenteditable` editor locator. The editor MUST be focused and
 *  contain the original content before the caller types. */
async function openPageAndEditBlock(
  page: Page,
  pageName: string,
  expectedContent: string,
) {
  await page.goto(`${FRONTEND_URL}/page/${encodeURIComponent(pageName)}`)
  const readContent = page.locator('.block-content-read').first()
  await expect(readContent).toBeVisible({ timeout: 10_000 })
  await readContent.click()
  const editor = page.locator('.block-content[contenteditable="true"]').first()
  await expect(editor).toBeVisible({ timeout: 5_000 })
  // The editor mounts with the original block content; if the
  // expected text isn't there, the seed never landed.
  await expect(editor).toContainText(expectedContent)
  return editor
}

/** Clear the editor's text and type the given string at the start.
 *  The slash menu keys off the editor's textContent — the new text
 *  MUST start with `/`, so we always replace, never append. We use
 *  `fill('')` to wipe (sets textContent + fires React's input event)
 *  and then `type()` with a small delay so the input handler
 *  processes each character individually. */
async function replaceEditorText(
  editor: import('@playwright/test').Locator,
  text: string,
) {
  await editor.fill('')
  await editor.type(text, { delay: 10 })
}

/** Type the slash command and confirm it via the keyboard, not the
 *  mouse. The click path is racy in E2E: clicking a menu item fires
 *  mousedown → editor blur → React unmounts the menu → the click
 *  event has no target left to fire on. The SlashCommandMenu's
 *  document-level keydown handler picks Enter for the highlighted
 *  item regardless, so `press('Enter')` is the deterministic path.
 *
 *  `arrowDownCount` lets callers skip past items that match the
 *  query but aren't the desired one (e.g. `/task` matches
 *  status-todo first, then task-role; pass 1 to select the second). */
async function applySlashCommand(
  editor: import('@playwright/test').Locator,
  slashText: string,
  arrowDownCount = 0,
) {
  await replaceEditorText(editor, slashText)
  // Sanity: the menu must be visible after the slash. We use the
  // menu's fixed-position container (z-index 200) as the signal
  // because which category headers render depends on which items
  // match the query — `/card` and `/query` show only the "Roles"
  // header, `/h1` shows only the "Block Types" header, etc.
  const menu = editor.page().locator('div[style*="z-index: 200"]')
  await expect(menu).toBeVisible({ timeout: 5_000 })
  for (let i = 0; i < arrowDownCount; i++) {
    await editor.press('ArrowDown')
  }
  await editor.press('Enter')
}

// ── Tests ────────────────────────────────────────────────────────

test.describe('Slash menu @slash-commands', () => {
  test('@smoke slash menu opens on / key @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-menu-open-${s}`
    await createPage(page, host)
    await createBlock(page, host, 'seed-for-slash-open')

    const editor = await openPageAndEditBlock(page, host, 'seed-for-slash-open')
    await replaceEditorText(editor, '/')

    // The menu is rendered as a fixed-position div with z-index 200.
    // Its visibility is the strongest "menu is open" signal —
    // category headers (Status, Roles, ...) only render when their
    // category has matching items, so a single header is fragile.
    const menu = page.locator('div[style*="z-index: 200"]')
    await expect(menu).toBeVisible({ timeout: 5_000 })
    // The first status item "TODO" is rendered as a label inside
    // the menu. With query='' the Status section shows the six
    // status items. Use `.first()` because the registry's render
    // path iterates categories × items and can render the same
    // item under multiple category containers — a single label
    // text can therefore match more than once in the DOM.
    await expect(menu.getByText('TODO', { exact: true }).first()).toBeVisible()
  })

  test('slash menu closes on Escape @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-menu-escape-${s}`
    await createPage(page, host)
    await createBlock(page, host, 'seed-for-escape')

    const editor = await openPageAndEditBlock(page, host, 'seed-for-escape')
    await replaceEditorText(editor, '/')

    // Sanity: menu is open before Escape.
    const menu = page.locator('div[style*="z-index: 200"]')
    await expect(menu).toBeVisible({ timeout: 5_000 })

    // Escape is handled at document level by SlashCommandMenu.
    await page.keyboard.press('Escape')

    // The container unmounts when slash state clears.
    await expect(menu).toBeHidden({ timeout: 5_000 })
  })

  test('slash menu filters by typed text (headings) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-menu-filter-${s}`
    await createPage(page, host)
    await createBlock(page, host, 'seed-for-filter')

    const editor = await openPageAndEditBlock(page, host, 'seed-for-filter')
    // Typing `/h` opens the menu with the heading items among the
    // matches (the registry's `heading1/2/3` items have "Heading N" in
    // their label — they are guaranteed to be present in the filtered
    // list). We do NOT assert that ONLY the 3 headings are shown,
    // because other items with "h" in keywords (priority A → "high",
    // priority C → "have", query → "search", card → "shape", etc.)
    // also match. The test asserts the headings ARE present.
    //
    // Use `.first()` on each label because the menu's render
    // iterates categories × items — each item can render under
    // multiple category containers in the DOM, so a single label
    // text can match more than once.
    await replaceEditorText(editor, '/h')

    await expect(
      page.getByText('Heading 1', { exact: true }).first(),
    ).toBeVisible({ timeout: 5_000 })
    await expect(
      page.getByText('Heading 2', { exact: true }).first(),
    ).toBeVisible()
    await expect(
      page.getByText('Heading 3', { exact: true }).first(),
    ).toBeVisible()
  })
})

test.describe('Status slash commands @slash-commands', () => {
  // The status handler in `slashRegistry.tsx` calls
  // `api.updateBlock(id, { marker: TaskMarker })` and the server's
  // PATCH handler accepts and persists both `marker` and
  // `priority`. The tests assert BOTH the DOM (marker badge
  // visible) and the API (block record carries the title-cased
  // marker — Todo / Done / Cancelled) — the round-trip matters
  // because a silent server-side drop would leave the badge
  // missing on reload, even if the client rendered it
  // optimistically.
  test('/todo sets block marker to Todo @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-status-todo-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'todo seed')

    const editor = await openPageAndEditBlock(page, host, 'todo seed')
    // /todo matches status-todo, the blockType `todo`, and task-role.
    // The first match (status-todo) is the one we want, so no
    // ArrowDown needed.
    await applySlashCommand(editor, '/todo', 0)

    // The marker badge renders the marker as UPPERCASE text inside
    // the block row (see BlockRow.tsx line 1177). The status handler
    // sets `marker: "Todo"` (title-cased) on the server.
    const row = page.locator('.block-row').first()
    await expect(row.getByText('TODO', { exact: true })).toBeVisible({
      timeout: 5_000,
    })

    // Server-side confirmation: the block record carries marker=Todo.
    // (TaskMarker is the title-cased enum value, not the lowercase id.)
    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.marker).toBe('Todo')

    await deleteAllBlocks(page, host)
  })

  test('/done sets block marker to Done @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-status-done-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'done seed')

    const editor = await openPageAndEditBlock(page, host, 'done seed')
    await applySlashCommand(editor, '/done', 0)

    const row = page.locator('.block-row').first()
    await expect(row.getByText('DONE', { exact: true })).toBeVisible({
      timeout: 5_000,
    })

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.marker).toBe('Done')

    await deleteAllBlocks(page, host)
  })

  test('/cancelled sets block marker to Cancelled @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-status-cancelled-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'cancelled seed')

    const editor = await openPageAndEditBlock(page, host, 'cancelled seed')
    await applySlashCommand(editor, '/cancelled', 0)

    const row = page.locator('.block-row').first()
    await expect(row.getByText('CANCELLED', { exact: true })).toBeVisible({
      timeout: 5_000,
    })

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.marker).toBe('Cancelled')

    await deleteAllBlocks(page, host)
  })
})

// ─── Block-to-task conversion (ADR-0023 deviation, ADR-0025) ─────────────
//
// When a marker is set on a paragraph/bullet/numbered block via slash
// command, blockType converts to 'todo' in the same API call. This is a
// ONE-WAY conversion: clearing the marker does NOT revert blockType.

test.describe('Block-to-task conversion — one-way paragraph→todo @slash-commands', () => {
  test('/TODO on paragraph block → blockType:"todo" AND marker:"Todo" in single update', async ({ page }) => {
    const s = suffix()
    const host = `block-to-task-para-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'paragraph to task')

    // Verify initial blockType is 'paragraph'
    const blocks0 = await getPageBlocks(page, host)
    const initial = blocks0.find((b) => b.id === blockId)
    expect(initial?.blockType).toBe('paragraph')

    const editor = await openPageAndEditBlock(page, host, 'paragraph to task')
    await applySlashCommand(editor, '/todo', 0)

    // Server-side: blockType MUST be 'todo' AND marker MUST be 'Todo'
    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType, 'blockType should be todo').toBe('todo')
    expect(updated!.marker, 'marker should be Todo').toBe('Todo')

    await deleteAllBlocks(page, host)
  })

  test('/DONE on heading1 block → blockType:"todo" AND marker:"Done" (ADR-0025 deviation)', async ({ page }) => {
    const s = suffix()
    const host = `block-to-task-heading-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'heading to task')

    // Set blockType to heading1 first (via API)
    const headers = getAuthHeaders()
    await page.request.patch(`${API_URL}/api/v1/blocks/${blockId}`, {
      data: { blockType: 'heading1' },
      headers,
    })

    const editor = await openPageAndEditBlock(page, host, 'heading to task')
    await applySlashCommand(editor, '/done', 0)

    // Server-side: blockType converts to 'todo' AND marker is 'Done'
    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType, 'blockType should be todo after /DONE on heading').toBe('todo')
    expect(updated!.marker, 'marker should be Done').toBe('Done')

    await deleteAllBlocks(page, host)
  })

  test('clearing marker on todo block → blockType stays "todo" (one-way, ADR-0025)', async ({ page }) => {
    const s = suffix()
    const host = `block-to-task-clear-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'clear marker test')

    // First set blockType to todo and marker to Todo
    const headers = getAuthHeaders()
    await page.request.patch(`${API_URL}/api/v1/blocks/${blockId}`, {
      data: { blockType: 'todo', marker: 'Todo' },
      headers,
    })

    // Verify it's set correctly
    const blocks0 = await getPageBlocks(page, host)
    const initial = blocks0.find((b) => b.id === blockId)
    expect(initial?.blockType).toBe('todo')
    expect(initial?.marker).toBe('Todo')

    // Now clear the marker using the API directly
    await page.request.patch(`${API_URL}/api/v1/blocks/${blockId}`, {
      data: { marker: null },
      headers,
    })

    // BlockType should STILL be 'todo' — this is the one-way conversion
    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType, 'blockType should stay todo after clearing marker (one-way)').toBe('todo')
    expect(updated!.marker, 'marker should be null').toBeNull()

    await deleteAllBlocks(page, host)
  })
})

test.describe('Block type slash commands @slash-commands', () => {
  test('/h1 creates heading1 block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-h1-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'h1 seed')

    const editor = await openPageAndEditBlock(page, host, 'h1 seed')
    await applySlashCommand(editor, '/h1', 0)

    // The slash handler clears the editor text (preserveContent is
    // false for blockType changes) and PATCHes blockType=heading1.
    // We assert the API record first because it's a stronger signal
    // than the DOM (the blockType renders visually but the exact
    // CSS differs across themes).
    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('heading1')

    // Reload the page and re-read the API. The blockType MUST survive
    // a fresh server response — this is the regression guard for
    // P0 fix #6 (slash blockType used to be lost on save).
    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded, 'block should still exist after reload').toBeDefined()
    expect(reloaded!.blockType).toBe('heading1')

    await deleteAllBlocks(page, host)
  })

  test('/h2 creates heading2 block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-h2-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'h2 seed')

    const editor = await openPageAndEditBlock(page, host, 'h2 seed')
    await applySlashCommand(editor, '/h2', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('heading2')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('heading2')

    await deleteAllBlocks(page, host)
  })

  test('/code creates code block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-code-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'code seed')

    const editor = await openPageAndEditBlock(page, host, 'code seed')
    await applySlashCommand(editor, '/code', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('code')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('code')

    await deleteAllBlocks(page, host)
  })

  test('/quote creates quote block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-quote-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'quote seed')

    const editor = await openPageAndEditBlock(page, host, 'quote seed')
    await applySlashCommand(editor, '/quote', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('quote')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('quote')

    await deleteAllBlocks(page, host)
  })
})

test.describe('Role slash commands @slash-commands', () => {
  test('/task sets type:: task + status:: todo @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-role-task-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'task seed')

    const editor = await openPageAndEditBlock(page, host, 'task seed')
    // /task matches status-todo, task-role, and the blockType `todo`.
    // status-todo is registered first, so Enter would select it. We
    // need task-role, which is the 2nd item — press ArrowDown once to
    // advance the highlight, then Enter.
    await applySlashCommand(editor, '/task', 1)

    // The role handler is a property transform: it does NOT change
    // blockType and it does NOT change marker. The block's
    // blockType stays paragraph; the change lives in `properties`.
    // The handler does:
    //   await api.setBlockProperty(id, 'type', 'task')
    //   await api.setBlockProperty(id, 'status', 'todo')
    // (see slashRegistry.tsx — makeRolePropertiesHandler)
    //
    // Both PUTs are async, so we retry the assertion until the
    // second one lands. The window is short (the handler awaits
    // each PUT sequentially) so 1s of polling is plenty.
    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toMatchObject({
        blockType: 'paragraph',
        properties: { type: 'task', status: 'todo' },
      })

    await deleteAllBlocks(page, host)
  })

  test('/query sets type:: query + dsl:: @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-role-query-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'query seed')

    // The /query handler calls window.prompt('Enter a DSL query:').
    // Accept the dialog with a fixed DSL string.
    const expectedDsl = `(and (page-property status open))`
    page.once('dialog', async (dialog) => {
      expect(dialog.type()).toBe('prompt')
      await dialog.accept(expectedDsl)
    })

    const editor = await openPageAndEditBlock(page, host, 'query seed')
    // /query is uniquely matched by query-role (no other item label,
    // blockType, or keyword contains "query"). Enter selects it,
    // which calls window.prompt and the dialog handler above accepts
    // with the expected DSL string.
    await applySlashCommand(editor, '/query', 0)

    // The handler does:
    //   await api.setBlockProperty(id, 'type', 'query')
    //   await api.setBlockProperty(id, 'dsl', dsl)
    // (the second PUT races the test's read — poll until both land)
    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toMatchObject({
        blockType: 'paragraph',
        properties: { type: 'query', dsl: expectedDsl },
      })

    await deleteAllBlocks(page, host)
  })

  test('/card sets card-shape:: @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-role-card-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'card seed')

    // The /card handler prompts for a card shape (one of:
    // content, reference, presentation, article, note). Default
    // is "content" if the user dismisses or types an unknown value.
    page.once('dialog', async (dialog) => {
      expect(dialog.type()).toBe('prompt')
      await dialog.accept('reference')
    })

    const editor = await openPageAndEditBlock(page, host, 'card seed')
    // /card is uniquely matched by card-role. Enter selects it,
    // which calls window.prompt and the dialog handler above
    // accepts with 'reference'.
    await applySlashCommand(editor, '/card', 0)

    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toMatchObject({
        blockType: 'paragraph',
        properties: { 'card-shape': 'reference' },
      })

    await deleteAllBlocks(page, host)
  })
})

// ─── More status slash commands ──────────────────────────────────────
//
// The status menu in `slashRegistry.tsx` registers SIX markers:
// TODO, DOING, DONE, NOW, LATER, CANCELLED. The first three are
// covered by the `Status slash commands` describe block above; the
// remaining three are exercised here.
//
// The `/doing` command: the backend's `TaskMarker::from_str`
// has always accepted "doing" (case-insensitive). The slash command
// itself works. The latent bugs fixed in this change were:
//   (a) pest grammar omitted "doing" — DSL query `(task doing)` failed
//   (b) error message listed only 5 markers, omitting "doing"
//   (c) heuristic rules generated "in-progress" instead of "doing"
// The test below asserts the documented contract (marker='Doing').

test.describe('More status slash commands @slash-commands', () => {
  test('/doing attempts to set block marker to Doing @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-status-doing-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'doing seed')

    const editor = await openPageAndEditBlock(page, host, 'doing seed')
    // /doing matches only `status-doing` — "doing" appears in
    // `status-doing` keywords and no other item's label/blockType/
    // keywords. Enter selects it directly.
    await applySlashCommand(editor, '/doing', 0)

    // The backend now supports 'Doing' as a TaskMarker variant.
    // The marker badge renders the title-cased marker as UPPERCASE
    // text inside the block row.
    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toMatchObject({ marker: 'Doing' })

    await deleteAllBlocks(page, host)
  })

  test('/now sets block marker to Now @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-status-now-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'now seed')

    const editor = await openPageAndEditBlock(page, host, 'now seed')
    // /now matches only `status-now` (no other item has "now" in
    // its label/blockType/keywords).
    await applySlashCommand(editor, '/now', 0)

    // The marker badge renders the title-cased marker as UPPERCASE
    // text inside the block row.
    const row = page.locator('.block-row').first()
    await expect(row.getByText('NOW', { exact: true })).toBeVisible({
      timeout: 5_000,
    })

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.marker).toBe('Now')

    await deleteAllBlocks(page, host)
  })

  test('/later sets block marker to Later @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-status-later-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'later seed')

    const editor = await openPageAndEditBlock(page, host, 'later seed')
    // /later matches only `status-later`.
    await applySlashCommand(editor, '/later', 0)

    const row = page.locator('.block-row').first()
    await expect(row.getByText('LATER', { exact: true })).toBeVisible({
      timeout: 5_000,
    })

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.marker).toBe('Later')

    await deleteAllBlocks(page, host)
  })

  test('/waiting sets block marker to Waiting @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-status-waiting-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'waiting seed')

    const editor = await openPageAndEditBlock(page, host, 'waiting seed')
    // /waiting matches only `status-waiting`.
    await applySlashCommand(editor, '/waiting', 0)

    // The marker badge renders the title-cased marker as UPPERCASE
    // text inside the block row.
    const row = page.locator('.block-row').first()
    await expect(row.getByText('WAITING', { exact: true })).toBeVisible({
      timeout: 5_000,
    })

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.marker).toBe('Waiting')

    await deleteAllBlocks(page, host)
  })
})

// ─── Priority slash commands ─────────────────────────────────────────

test.describe('Priority slash commands @slash-commands', () => {
  // The `makePriorityHandler` in slashRegistry.tsx calls
  // `api.updateBlock(id, { priority: 'A' | 'B' | 'C' })` (the Rust
  // `UpdateBlockRequest` accepts `priority` as a `Priority` enum
  // variant). The TaskMarker badge does NOT render priority — the
  // assertion goes through the API only, using a polling loop to
  // tolerate the round-trip latency.
  test('/priority A sets block priority to A @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-priority-a-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'priority-a seed')

    const editor = await openPageAndEditBlock(page, host, 'priority-a seed')
    // The slash menu registers the priorities as items with
    // labels "Priority A", "Priority B", "Priority C". The query
    // "/priority A" lowercases to "priority a" — the substring
    // "priority a" only appears in the `Priority A` item's label
    // (lowercased). Other items with "a" in keywords (e.g.
    // priority-a itself with keyword "a") don't include the
    // multi-word substring, so they don't match. Enter selects
    // Priority A directly.
    await applySlashCommand(editor, '/priority A', 0)

    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toMatchObject({ priority: 'A' })

    await deleteAllBlocks(page, host)
  })

  test('/priority B sets block priority to B @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-priority-b-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'priority-b seed')

    const editor = await openPageAndEditBlock(page, host, 'priority-b seed')
    await applySlashCommand(editor, '/priority B', 0)

    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toMatchObject({ priority: 'B' })

    await deleteAllBlocks(page, host)
  })

  test('/priority C sets block priority to C @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-priority-c-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'priority-c seed')

    const editor = await openPageAndEditBlock(page, host, 'priority-c seed')
    await applySlashCommand(editor, '/priority C', 0)

    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toMatchObject({ priority: 'C' })

    await deleteAllBlocks(page, host)
  })
})

// ─── Date slash commands ─────────────────────────────────────────────

test.describe('Date slash commands @slash-commands', () => {
  // The `makeDateHandler` in slashRegistry.tsx calls
  // `ctx.setContent(dateStr)` where `dateStr` is the ISO date
  // (YYYY-MM-DD) for today or tomorrow. The handler does NOT
  // touch the block record itself — it edits the editor content,
  // which the debounced save then PATCHes back. We assert via
  // the API after a polling window.
  test('/today inserts todays date as block content @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-date-today-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'date-today seed')

    const editor = await openPageAndEditBlock(page, host, 'date-today seed')
    // /today matches TWO items in registration order:
    //   1. `status-todo` (keywords include "todo", which is a
    //      substring of "today" — but is it? "today" = t-o-d-a-y;
    //      "todo" = t-o-d-o; the 4th char differs, so "todo" is NOT
    //      a substring of "today". So status-todo does NOT match.)
    //   2. `date-today` (label "Today" → matches "today").
    // Status-todo's keyword "todo" is checked via
    // `k.includes(q)` where `q = "today"` and `k = "todo"`. "todo"
    // does not include "today" (the strings diverge at index 3).
    // So only `date-today` matches. Enter selects it directly.
    await applySlashCommand(editor, '/today', 0)

    // The handler calls `setContent(today)` which (a) updates the
    // editor's textContent and (b) debounces a save of the new
    // content. Wait for the PATCH to land and the GET to return
    // the date string. The date is computed at handler-time in
    // the browser, so we recompute it here to compare.
    const expected = new Date().toISOString().split('T')[0]
    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)?.content
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toBe(expected)

    await deleteAllBlocks(page, host)
  })

  test('/tomorrow inserts tomorrows date as block content @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-date-tomorrow-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'date-tomorrow seed')

    const editor = await openPageAndEditBlock(page, host, 'date-tomorrow seed')
    // /tomorrow matches only `date-tomorrow` (label "Tomorrow").
    await applySlashCommand(editor, '/tomorrow', 0)

    const expected = new Date(Date.now() + 86400000).toISOString().split('T')[0]
    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)?.content
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toBe(expected)

    await deleteAllBlocks(page, host)
  })
})

// ─── Date property slash commands ────────────────────────────────────

test.describe('Date property slash commands @slash-commands', () => {
  // `makePropertyHandler` inserts a property-syntax fragment like
  // `deadline:: ` or `scheduled:: ` into the editor's textContent
  // and moves the cursor to the end. The PATCH that follows is a
  // plain content save (no properties key), so the API record's
  // `content` field carries the fragment. We assert against the
  // content (NOT the structured `properties` array) because the
  // colon-double-colon syntax is the user-visible contract — the
  // properties parser runs later in the lifecycle.
  test('/deadline inserts deadline::  in block content @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-prop-deadline-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'deadline seed')

    const editor = await openPageAndEditBlock(page, host, 'deadline seed')
    // /deadline matches only `prop-deadline` (label "Deadline").
    await applySlashCommand(editor, '/deadline', 0)

    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)?.content
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toBe('deadline:: ')

    await deleteAllBlocks(page, host)
  })

  test('/scheduled inserts scheduled::  in block content @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-prop-scheduled-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'scheduled seed')

    const editor = await openPageAndEditBlock(page, host, 'scheduled seed')
    // /scheduled matches only `prop-scheduled` (label "Scheduled").
    await applySlashCommand(editor, '/scheduled', 0)

    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)?.content
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toBe('scheduled:: ')

    await deleteAllBlocks(page, host)
  })
})

// ─── Reference slash commands ────────────────────────────────────────

test.describe('Reference slash commands @slash-commands', () => {
  // `makeRefHandler` writes `[[` (page reference) or `((` (block
  // embed) to the editor's textContent and moves the cursor to
  // the end. The textContent is then debounced-saved to the API,
  // so we poll for the content to land.
  test('/page reference inserts [[  in block content @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-ref-page-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'ref-page seed')

    const editor = await openPageAndEditBlock(page, host, 'ref-page seed')
    // /page reference matches only `ref-page` (label
    // "Page Reference" lowercased contains "page reference").
    await applySlashCommand(editor, '/page reference', 0)

    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)?.content
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toBe('[[')

    await deleteAllBlocks(page, host)
  })

  test('/block embed inserts ((  in block content @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-ref-block-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'ref-block seed')

    const editor = await openPageAndEditBlock(page, host, 'ref-block seed')
    // /block embed matches only `ref-block` (label "Block Embed"
    // lowercased contains "block embed").
    await applySlashCommand(editor, '/block embed', 0)

    await expect
      .poll(async () => {
        const blocks = await getPageBlocks(page, host)
        return blocks.find((b) => b.id === blockId)?.content
      }, { timeout: 5_000, intervals: [100, 200, 500] })
      .toBe('((')

    await deleteAllBlocks(page, host)
  })
})

// ─── Template slash command ──────────────────────────────────────────

test.describe('Template slash command @slash-commands', () => {
  // `/new from template` (and `/template`) trigger the
  // `handleInsertTemplate` wizard (BlockRow.tsx:992), which:
  //   1. Prompts `window.prompt('New page name:')` — we accept
  //      with a unique name.
  //   2. If multiple templates exist, prompts again with the
  //      list — we accept with the FIRST template's name.
  //   3. Calls `api.createPageFromTemplate(...)` → 201.
  //   4. Navigates to the new page.
  //
  // The test asserts the API side: a new page with the chosen
  // name appears after the command runs. We do not assert the
  // SPA navigation (TanStack Router) because the dev server's
  // Vite proxy can be flaky on the very first cross-page nav;
  // the API ground truth is the stronger signal.
  test('/new from template creates a page from a template @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-template-${s}`
    await createPage(page, host)
    await createBlock(page, host, 'template seed')

    // Pre-fetch the template list so we can answer the second
    // prompt deterministically (the wizard prompts for a
    // template name when there's more than one).
    const headers = getAuthHeaders()
    const listResp = await page.request.get(`${API_URL}/api/v1/templates`, { headers })
    expect(listResp.ok(), 'templates endpoint must respond').toBe(true)
    const templates = (await listResp.json()) as Array<{ name: string; full_name: string }>
    expect(templates.length, 'at least one template must exist').toBeGreaterThan(0)
    const firstTemplateName = templates[0].name

    // New page name — unique to avoid collisions with the global
    // `quilt-pages` table.
    const newPageName = `slash-template-page-${s}`

    // The slash command fires TWO window.prompts in sequence
    // (name → template). We register two listeners; Playwright
    // fires them in registration order.
    let promptCount = 0
    page.on('dialog', async (dialog) => {
      promptCount += 1
      expect(dialog.type()).toBe('prompt')
      if (promptCount === 1) {
        await dialog.accept(newPageName)
      } else {
        await dialog.accept(firstTemplateName)
      }
    })

    const editor = await openPageAndEditBlock(page, host, 'template seed')
    // /new from template matches only `insert-template` (label
    // "New from Template" + keyword "new from template"). The
    // registry registers it as the 16th item.
    await applySlashCommand(editor, '/new from template', 0)

    // The wizard navigates the browser to the new page. Either
    // navigation succeeds (URL changes) or the API call
    // succeeds — both are evidence the command worked. We poll
    // the API for the new page's existence.
    await expect
      .poll(
        async () => {
          const resp = await page.request.get(
            `${API_URL}/api/v1/pages/${encodeURIComponent(newPageName)}`,
            { headers },
          )
          return resp.ok()
        },
        { timeout: 10_000, intervals: [200, 500, 1000] },
      )
      .toBe(true)

    // Cleanup: delete the newly-created page so the global
    // namespace stays clean for parallel runs.
    await page.request.delete(`${API_URL}/api/v1/pages/${encodeURIComponent(newPageName)}`, { headers })
    await deleteAllBlocks(page, host)
  })
})

// ─── Comment slash command ───────────────────────────────────────────

test.describe('Comment slash command @slash-commands', () => {
  // `/add comment` triggers `defaultCommentHandler`, which calls
  // `onAddComment(blockId)` — the BlockRow prop wired by PageView
  // to `handleAddComment` (PageView.tsx:1486). That handler
  // prompts `window.prompt('Add comment:')` and, on accept, calls
  // `api.createBlock({ ..., parentId: blockId, properties:
  // { type: 'comment', ... } })`. The new child block is
  // persisted as a sibling comment under the target.
  test('/add comment creates a child block with type=comment @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-comment-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'comment seed')

    // Accept the comment-text prompt with a known value.
    const commentText = `e2e-comment-${s}`
    page.once('dialog', async (dialog) => {
      expect(dialog.type()).toBe('prompt')
      await dialog.accept(commentText)
    })

    const editor = await openPageAndEditBlock(page, host, 'comment seed')
    // /add comment matches only `add-comment` (label "Add Comment").
    await applySlashCommand(editor, '/add comment', 0)

    // The handler creates a child block with `type:: comment` and
    // `parentId === blockId`. Poll until the new block lands.
    await expect
      .poll(
        async () => {
          const blocks = await getPageBlocks(page, host)
          return blocks.find(
            (b) => b.parentId === blockId && b.content === commentText,
          )
        },
        { timeout: 5_000, intervals: [100, 200, 500] },
      )
      .toBeDefined()

    // Verify the `type: comment` property is set on the child.
    const blocks = await getPageBlocks(page, host)
    const comment = blocks.find(
      (b) => b.parentId === blockId && b.content === commentText,
    )!
    expect(comment.properties, 'comment must carry type=comment').toMatchObject({
      type: 'comment',
    })

    await deleteAllBlocks(page, host)
  })
})

// ─── More block type slash commands ──────────────────────────────────

test.describe('More block type slash commands @slash-commands', () => {
  // These mirror the existing `/h1` / `/h2` / `/code` / `/quote`
  // tests: apply the command, assert the API blockType, reload
  // the page, assert the blockType PERSISTS. The persistence
  // assertion is the regression guard for P0 fix #6 — slash
  // blockType was lost on save before that fix.
  test('/text resets blockType to paragraph @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-text-${s}`
    await createPage(page, host)
    // Seed the block as heading1 via the API so the test starts
    // from a non-paragraph state — otherwise `/text` on an
    // already-paragraph block would be a no-op from the API's
    // perspective. The PATCH that flips it back to paragraph is
    // the only observable change.
    const blockId = await createBlock(page, host, 'text seed')
    const headers = getAuthHeaders()
    const seedResp = await page.request.patch(`${API_URL}/api/v1/blocks/${blockId}`, {
      data: { blockType: 'heading1' },
      headers,
    })
    expect(seedResp.ok(), 'seed PATCH must succeed').toBe(true)

    const editor = await openPageAndEditBlock(page, host, 'text seed')
    // /text matches only `paragraph` (label "Text", keywords
    // include "text"). No other item has "text" in its
    // label/blockType/keywords.
    await applySlashCommand(editor, '/text', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('paragraph')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded, 'block should still exist after reload').toBeDefined()
    expect(reloaded!.blockType).toBe('paragraph')

    await deleteAllBlocks(page, host)
  })

  test('/h3 creates heading3 block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-h3-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'h3 seed')

    const editor = await openPageAndEditBlock(page, host, 'h3 seed')
    // /h3 matches only `heading3` (keywords include "h3").
    await applySlashCommand(editor, '/h3', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('heading3')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('heading3')

    await deleteAllBlocks(page, host)
  })

  test('/bullet creates bullet list block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-bullet-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'bullet seed')

    const editor = await openPageAndEditBlock(page, host, 'bullet seed')
    // /bullet matches ONLY `bullet` (label "Bullet List", blockType
    // "bullet", keywords include "bullet"). The `quote` item's
    // keyword "blockquote" is checked with `k.includes(q)` where
    // `q = "bullet"` and `k = "blockquote"`. "blockquote" is
    // b-l-o-c-k-q-u-o-t-e — the second char is `l`, not `u`, so
    // the substring match fails. Enter selects `bullet` directly.
    await applySlashCommand(editor, '/bullet', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('bullet')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('bullet')

    await deleteAllBlocks(page, host)
  })

  test('/numbered creates numbered list block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-numbered-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'numbered seed')

    const editor = await openPageAndEditBlock(page, host, 'numbered seed')
    // /numbered matches only `numbered` (label "Numbered List",
    // blockType "numbered", keywords include "numbered").
    await applySlashCommand(editor, '/numbered', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('numbered')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('numbered')

    await deleteAllBlocks(page, host)
  })

  test('/divider creates divider block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-divider-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'divider seed')

    const editor = await openPageAndEditBlock(page, host, 'divider seed')
    // /divider matches only `divider` (label "Divider", blockType
    // "divider", keywords include "divider").
    await applySlashCommand(editor, '/divider', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('divider')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('divider')

    await deleteAllBlocks(page, host)
  })

  test('/image creates image block (persists across reload) @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-image-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'image seed')

    const editor = await openPageAndEditBlock(page, host, 'image seed')
    // /image matches only `image` (label "Image", blockType
    // "image", keywords include "image").
    await applySlashCommand(editor, '/image', 0)

    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('image')

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('image')

    await deleteAllBlocks(page, host)
  })

  test('/todo as block type (not status) sets blockType to todo @slash-commands', async ({ page }) => {
    const s = suffix()
    const host = `slash-type-todo-${s}`
    await createPage(page, host)
    const blockId = await createBlock(page, host, 'todo-type seed')

    const editor = await openPageAndEditBlock(page, host, 'todo-type seed')
    // /todo matches THREE items: status-todo (index 0),
    // task-role (index 17), and `todo` block type (index 26).
    // The first test in the `Status slash commands` describe
    // block already exercises the status-todo path with
    // arrowDownCount=0. To select the blockType variant, we
    // press ArrowDown twice: 0 → status-todo, 1 → task-role,
    // 2 → todo block type.
    await applySlashCommand(editor, '/todo', 2)

    // The blockType handler is `defaultBlockTypeHandler` — it
    // PATCHes blockType='todo' and does NOT touch the marker.
    // So the block ends up with blockType=todo, marker=null
    // (no marker badge in the UI).
    const blocks = await getPageBlocks(page, host)
    const updated = blocks.find((b) => b.id === blockId)
    expect(updated, 'block should still exist').toBeDefined()
    expect(updated!.blockType).toBe('todo')
    expect(updated!.marker, 'blockType slash must not set a marker').toBeNull()

    await page.reload()
    await expect(page.locator('.block-content-read').first()).toBeVisible({
      timeout: 10_000,
    })
    const blocksAfterReload = await getPageBlocks(page, host)
    const reloaded = blocksAfterReload.find((b) => b.id === blockId)
    expect(reloaded!.blockType).toBe('todo')

    await deleteAllBlocks(page, host)
  })
})
