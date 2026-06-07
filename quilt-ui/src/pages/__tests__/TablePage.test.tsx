// TablePage — loads available property keys via the property-keys endpoint.
//
// Frontend fix (issue #8): the page used to call `api.getBlockProperties('')`
// as a hack to enumerate every property key for the filter-chip dropdown.
// The correct endpoint is `GET /api/v1/properties/keys` (mounted in
// `crates/quilt-server/src/routes.rs:40`).
//
// We assert the BEHAVIOR — the page calls `listPropertyKeys` and feeds
// the result into QueryBuilder as `availableKeys` — without spinning up
// the full query-builder machinery.

import { render, screen, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'

// ── Mocks ───────────────────────────────────────────────────────────

// `vi.mock` is hoisted, so the factory can't reference variables
// declared later. `vi.hoisted` runs the factory early enough to be
// referenced from inside the mock factory.
const mocks = vi.hoisted(() => ({
  listPropertyKeys: vi.fn(),
}))

vi.mock('@core/api-client', () => ({
  api: {
    listPropertyKeys: mocks.listPropertyKeys,
  },
}))

// Mock QueryBuilder so the test focuses on the page's own data flow.
// The mock exposes a `data-available-keys` attribute so we can assert
// what was passed in without rendering the whole filter-chip tree.
vi.mock('@features/query-builder/QueryBuilder', () => ({
  QueryBuilder: ({ availableKeys }: { availableKeys?: string[] }) => (
    <div
      data-testid="query-builder"
      data-available-keys={JSON.stringify(availableKeys ?? [])}
    />
  ),
}))

// Import AFTER mocks so the page binds to the mocked api.
import { TablePage } from '../TablePage'

// ── Helpers ─────────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
})

// ── Tests ───────────────────────────────────────────────────────────

describe('TablePage — property keys load via the dedicated endpoint', () => {
  it('fetches keys with listPropertyKeys (not getBlockProperties with an empty id)', async () => {
    mocks.listPropertyKeys.mockResolvedValueOnce({
      keys: ['status', 'priority'],
      nextCursor: null,
    })

    render(<TablePage />)

    await waitFor(() => {
      expect(mocks.listPropertyKeys).toHaveBeenCalledTimes(1)
    })
    // No arguments — the page should rely on the server's default
    // page size rather than hard-coding a limit.
    expect(mocks.listPropertyKeys).toHaveBeenCalledWith()
  })

  it('forwards the returned keys to QueryBuilder as availableKeys', async () => {
    mocks.listPropertyKeys.mockResolvedValueOnce({
      keys: ['beta', 'alpha', 'gamma'],
      nextCursor: null,
    })

    render(<TablePage />)

    const builder = await screen.findByTestId('query-builder')

    // The mock JSON-encodes the prop so the assertion is a plain
    // string compare. We expect the page to sort the keys for a
    // stable dropdown order (matches the pre-fix behavior).
    const passed = JSON.parse(builder.getAttribute('data-available-keys') ?? '[]')
    expect(passed).toEqual(['alpha', 'beta', 'gamma'])
  })

  it('renders an empty key list (does not crash) when the endpoint returns no keys', async () => {
    mocks.listPropertyKeys.mockResolvedValueOnce({
      keys: [],
      nextCursor: null,
    })

    render(<TablePage />)

    const builder = await screen.findByTestId('query-builder')
    const passed = JSON.parse(builder.getAttribute('data-available-keys') ?? '[]')
    expect(passed).toEqual([])
  })
})
