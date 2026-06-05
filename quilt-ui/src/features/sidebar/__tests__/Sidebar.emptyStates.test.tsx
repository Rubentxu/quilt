// ─── Sidebar — F1 + F2 of quilt-fase2-ux-empty-states ─────────────
//
// Approval tests for two new pieces of behaviour the orchestrator
// asked for:
//
//   F1 — "Páginas" section
//     - When there are MORE than 5 non-journal pages, the section
//       only shows the 5 most recent (by `createdAt` desc) and adds
//       a "Ver todas (N)" link that points at /pages.
//     - When there are 5 or fewer pages, the section shows them
//       all and does NOT show the "Ver todas" link (the link would
//       be noise).
//     - When there are 0 pages, the existing "No hay páginas
//       todavía" empty state still renders and now also surfaces a
//       "Ver todas" link (so the user can browse the rest, which
//       might be journals or pages they can't see in the list).
//
//   F2 — "Favoritos" section
//     - When there are 0 favorites, the section is now visible
//       (it used to be hidden) and renders the empty-state
//       message "Click the star on any page to favorite it".
//     - When there is at least 1 favorite, the existing list
//       still renders.
//
// The Sidebar component is large (541 lines) and pulls in a lot
// of dependencies (TanStack router, the API client, SSE, etc.).
// Rather than re-mock the world we exercise the two code paths
// that changed via the minimal slice that depends on them:
//
//   - `api.listPages()` (returns the in-memory pages list).
//   - `useLocation()` (we render with a fixed path).
//   - `useNavigate()` (called by the "Nueva página" button — we
//     don't exercise it here, but the mock satisfies the type).
//   - `localStorage` (for favorites — already shimmed in
//     test/setup.ts).
//
// The tests are intentionally narrow: they cover F1 and F2 only.
// The original behaviour is exercised by the smoke tests that
// exist elsewhere.

import { render, screen, waitFor, within } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { Sidebar } from '../Sidebar'

// ── Mocked dependencies ────────────────────────────────────────────

const mockListPages = vi.fn()
const mockListTemplates = vi.fn()
const mockNavigate = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    listPages: (...args: unknown[]) => mockListPages(...args),
    // The Sidebar also mounts <TemplateSection /> which calls
    // listTemplates. We never assert against it — return [] so it
    // settles into the empty-templates state without logging.
    listTemplates: (...args: unknown[]) => mockListTemplates(...args),
  },
}))

vi.mock('@tanstack/react-router', () => ({
  Link: ({
    to,
    children,
    style: _style,
    className: _className,
    ...rest
  }: {
    to: string
    children: React.ReactNode
    style?: React.CSSProperties
    className?: string
    [key: string]: unknown
  }) => (
    <a href={to} {...rest}>
      {children}
    </a>
  ),
  useNavigate: () => mockNavigate,
  useLocation: () => ({ pathname: '/page/whatever' }),
}))

vi.mock('@features/cognitive/AgentActivityPanel', () => ({
  AgentActivityPanel: () => <div data-testid="agent-activity-panel" />,
}))

beforeEach(() => {
  mockListPages.mockReset()
  mockListTemplates.mockReset()
  mockNavigate.mockReset()
  // The Sidebar mounts <TemplateSection /> which immediately calls
  // api.listTemplates. Returning an empty array keeps it in the
  // empty-templates branch and out of the assertions for F1/F2.
  mockListTemplates.mockResolvedValue([])
  localStorage.clear()
})

// ── Helpers ────────────────────────────────────────────────────────

function makePage(id: string, name: string, daysAgo: number) {
  return {
    id,
    name,
    title: null,
    journal: false,
    journalDay: null,
    createdAt: new Date(Date.now() - daysAgo * 24 * 60 * 60 * 1000).toISOString(),
  }
}

async function renderSidebarWith(pages: unknown[]) {
  mockListPages.mockResolvedValue(pages)
  render(<Sidebar collapsed={false} onOpenSearch={() => {}} />)
  // Flush the api resolution so the post-fetch render fires.
  await waitFor(() => {
    expect(mockListPages).toHaveBeenCalled()
  })
}

// ── F1 tests ────────────────────────────────────────────────────────

describe('Sidebar — F1 (Páginas: cap at 5 + "Ver todas" link)', () => {
  it('shows the 5 most recent pages and a "Ver todas (N)" link when there are more than 5', async () => {
    const pages = [
      makePage('p1', 'alpha', 0),
      makePage('p2', 'bravo', 1),
      makePage('p3', 'charlie', 2),
      makePage('p4', 'delta', 3),
      makePage('p5', 'echo', 4),
      makePage('p6', 'foxtrot', 5), // oldest — should be hidden
      makePage('p7', 'golf', 6),    // even older — should be hidden
    ]
    await renderSidebarWith(pages)

    // The 5 newest must be in the DOM.
    expect(await screen.findByText('alpha')).toBeInTheDocument()
    expect(screen.getByText('bravo')).toBeInTheDocument()
    expect(screen.getByText('charlie')).toBeInTheDocument()
    expect(screen.getByText('delta')).toBeInTheDocument()
    expect(screen.getByText('echo')).toBeInTheDocument()

    // The two oldest must NOT be present.
    expect(screen.queryByText('foxtrot')).not.toBeInTheDocument()
    expect(screen.queryByText('golf')).not.toBeInTheDocument()

    // The "Ver todas" link must be visible and point to /pages.
    const seeAll = screen.getByTestId('pages-see-all')
    expect(seeAll).toBeInTheDocument()
    expect(seeAll.tagName).toBe('A')
    expect(seeAll).toHaveAttribute('href', '/pages')
    // The label shows the total count so the user knows how much
    // they are about to browse.
    expect(seeAll).toHaveTextContent(/Ver todas \(7\)/)
  })

  it('does NOT show the "Ver todas" link when there are 5 or fewer pages', async () => {
    const pages = [
      makePage('p1', 'alpha', 0),
      makePage('p2', 'bravo', 1),
      makePage('p3', 'charlie', 2),
    ]
    await renderSidebarWith(pages)

    expect(await screen.findByText('alpha')).toBeInTheDocument()
    expect(screen.getByText('bravo')).toBeInTheDocument()
    expect(screen.getByText('charlie')).toBeInTheDocument()

    // Link would be noise with only 3 pages — must be absent.
    expect(screen.queryByTestId('pages-see-all')).not.toBeInTheDocument()
  })

  it('shows the "No hay páginas" empty state when there are 0 regular pages', async () => {
    // Only journals exist (the user's "34 journals" scenario).
    const pages = Array.from({ length: 5 }, (_, i) => ({
      ...makePage(`j${i}`, `2026-06-0${i + 1}`, i),
      journal: true,
    }))
    await renderSidebarWith(pages)

    // The empty state line + a "Ver todas" link.
    const emptyState = await screen.findByTestId('pages-empty')
    expect(emptyState).toBeInTheDocument()
    expect(within(emptyState).getByText(/no hay páginas todavía/i)).toBeInTheDocument()

    // The "Ver todas" link lives inside the empty state so the
    // user always has a way to find their content.
    const emptySeeAll = within(emptyState).getByTestId('pages-empty-see-all')
    expect(emptySeeAll).toBeInTheDocument()
    expect(emptySeeAll).toHaveAttribute('href', '/pages')
  })

  it('orders the 5 visible pages by createdAt descending (newest first)', async () => {
    // Intentionally shuffle the input: the component must sort
    // and not just take the first 5 of the response.
    const pages = [
      makePage('p1', 'oldest', 10),
      makePage('p2', 'alpha', 0),
      makePage('p3', 'bravo', 1),
      makePage('p4', 'charlie', 2),
      makePage('p5', 'delta', 3),
      makePage('p6', 'echo', 4),
      makePage('p7', 'foxtrot', 5),
    ]
    await renderSidebarWith(pages)

    const list = await screen.findByText('alpha').then(() =>
      // Find the Páginas list by walking the DOM: the Páginas
      // section header is the anchor. We instead assert by the
      // visible order: alpha (newest) > bravo > charlie > delta >
      // echo. foxtrot must be hidden. oldest is also hidden.
      screen.getByText('alpha').closest('ul')!,
    )
    const items = within(list).getAllByRole('listitem')
    const labels = items.map(li => li.textContent?.trim() ?? '')
    expect(labels).toEqual(['alpha', 'bravo', 'charlie', 'delta', 'echo'])
  })
})

// ── F2 tests ────────────────────────────────────────────────────────

describe('Sidebar — F2 (Favoritos: empty state shown when 0 favorites)', () => {
  it('renders the "Favoritos" section with the empty-state message when there are no favorites', async () => {
    await renderSidebarWith([
      makePage('p1', 'alpha', 0),
      makePage('p2', 'bravo', 1),
    ])

    // The section header is always present (the F2 change made
    // the section visible even with 0 favorites).
    expect(
      screen.getByRole('heading', { name: 'Favoritos' }),
    ).toBeInTheDocument()

    // Empty-state body — the actionable hint, not the section
    // title.
    const empty = await screen.findByTestId('favorites-empty')
    expect(empty).toBeInTheDocument()
    expect(empty).toHaveTextContent(/click the star on any page to favorite it/i)
  })

  it('still renders the favorite list when at least one page is favorited', async () => {
    // Pre-seed the localStorage with a favorite. The name must be
    // different from the regular pages so we can disambiguate the
    // two "Favoritos" and "Páginas" links in the DOM.
    localStorage.setItem('quilt-favorites', JSON.stringify(['zeta']))

    await renderSidebarWith([
      makePage('p1', 'alpha', 0),
      makePage('p2', 'bravo', 1),
      makePage('p3', 'zeta', 2),
    ])

    // The empty state is gone; the favorite row is there.
    await waitFor(() => {
      expect(screen.queryByTestId('favorites-empty')).not.toBeInTheDocument()
    })
    // The favorite should be a clickable link to /page/zeta. Use
    // a regex anchored on the "Favoritos" heading to scope the
    // search — the same name also lives in the Páginas list
    // because zeta is a regular page.
    const favoritosSection = screen
      .getByRole('heading', { name: 'Favoritos' })
      .closest('section')!
    const link = within(favoritosSection).getByRole('link', { name: /zeta/i })
    expect(link).toHaveAttribute('href', '/page/zeta')
  })
})
