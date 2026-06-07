// KanbanPage — loads available property keys via the property-keys endpoint.
//
// Frontend fix (issue #8): the page used to call `api.getBlockProperties('')`
// as a hack to enumerate every property key in the graph. The empty block
// ID sent a request to `/blocks//properties` which 404s. The correct
// endpoint is `GET /api/v1/properties/keys` (mounted in
// `crates/quilt-server/src/routes.rs:40`).
//
// We assert the BEHAVIOR — the page calls `listPropertyKeys` and renders
// the returned keys in the "Group by" dropdown — not the internal state
// machinery.

import { render, screen, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'

// ── Mocks ───────────────────────────────────────────────────────────

// `vi.mock` is hoisted, so the factory can't reference variables
// declared later. `vi.hoisted` runs the factory early enough to be
// referenced from inside the mock factory.
const mocks = vi.hoisted(() => ({
  listPages: vi.fn(),
  getPageBlocks: vi.fn(),
  listPropertyKeys: vi.fn(),
}))

vi.mock('@core/api-client', () => ({
  api: {
    listPages: mocks.listPages,
    getPageBlocks: mocks.getPageBlocks,
    listPropertyKeys: mocks.listPropertyKeys,
  },
}))

// Mock the heavy KanbanBoard (dnd-kit, drag handlers) so the test
// focuses on the page's own data-fetching + dropdown wiring.
vi.mock('@features/kanban/KanbanBoard', () => ({
  KanbanBoard: ({ propertyKey }: { propertyKey: string }) => (
    <div data-testid="kanban-board" data-property-key={propertyKey} />
  ),
}))

// Import AFTER mocks so the page binds to the mocked api.
import { KanbanPage } from '../KanbanPage'

// ── Helpers ─────────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
})

// ── Tests ───────────────────────────────────────────────────────────

describe('KanbanPage — property keys load via the dedicated endpoint', () => {
  it('fetches keys with listPropertyKeys (not getBlockProperties with an empty id)', async () => {
    // No blocks — just make the page settle without doing real work.
    mocks.listPages.mockResolvedValueOnce([])
    mocks.listPropertyKeys.mockResolvedValueOnce({
      keys: ['priority', 'status'],
      nextCursor: null,
    })

    render(<KanbanPage />)

    await waitFor(() => {
      // The page must call the new endpoint …
      expect(mocks.listPropertyKeys).toHaveBeenCalledTimes(1)
    })
    // … and it must call it with NO arguments — the server defaults the
    // cursor/limit. We don't want a hard-coded limit of 50 leaking into
    // the API client.
    expect(mocks.listPropertyKeys).toHaveBeenCalledWith()
  })

  it('renders the returned keys in the "Group by" dropdown (excluding card-shape/icon)', async () => {
    mocks.listPages.mockResolvedValueOnce([])
    mocks.listPropertyKeys.mockResolvedValueOnce({
      keys: ['card-shape', 'icon', 'priority', 'status'],
      nextCursor: null,
    })

    render(<KanbanPage />)

    // Wait for the dropdown to populate — `screen.findByRole` retries
    // until the option appears, so we don't need a hard sleep.
    const dropdown = await screen.findByRole('combobox')

    // The page filters out template-related keys; only user-relevant
    // keys should appear as <option> values.
    await waitFor(() => {
      expect(
        (dropdown as HTMLSelectElement).querySelectorAll('option'),
      ).toHaveLength(2)
    })

    const optionValues = Array.from(
      (dropdown as HTMLSelectElement).querySelectorAll('option'),
    ).map(o => (o as HTMLOptionElement).value)
    expect(optionValues).toEqual(['priority', 'status'])
  })

  it('falls back to an empty dropdown (does not crash) when the endpoint returns no keys', async () => {
    mocks.listPages.mockResolvedValueOnce([])
    mocks.listPropertyKeys.mockResolvedValueOnce({
      keys: [],
      nextCursor: null,
    })

    render(<KanbanPage />)

    // The "No blocks with properties" empty state appears when there
    // are no blocks. With an empty keys list and no blocks, that's
    // exactly what should render.
    const empty = await screen.findByText(/no blocks with properties/i)
    expect(empty).toBeInTheDocument()

    // And getPageBlocks was never called because there are no pages.
    expect(mocks.getPageBlocks).not.toHaveBeenCalled()
  })
})
