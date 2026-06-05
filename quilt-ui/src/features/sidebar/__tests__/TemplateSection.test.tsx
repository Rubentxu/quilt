// ─── TemplateSection — sidebar templates UX (PR 3) ───────────────
//
// Approval tests for the sidebar's "Plantillas" section. The section
// fetches `api.listTemplates()` once on mount, renders a list of
// template items, and on click creates a new page via
// `api.createPageFromTemplate({templateName, pageName})`, where
// `pageName` defaults to `<templateName>-1` (collision suffix added by
// the spec).
//
// Spec: openspec/changes/quilt-fase1-sidebar-mcp-templates/specs/
//       sidebar-template-ux/spec.md (sidebar-template-ux capability).
// Design: design.md §D5 (positioned after Pages in the sidebar nav),
//         §D6 (single fetch on mount, AbortController on unmount).
//
// We mock the api-client and TanStack router because TemplateSection
// only needs `useNavigate` from the router — no full router instance.

import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import toast from 'react-hot-toast'

// Component under test. Imported AFTER the vi.mock declarations so the
// hoisted mocks are in place when the module graph is evaluated.
import { TemplateSection } from '../sections/TemplateSection'

// ── Mocked dependencies ────────────────────────────────────────────
// We mock the api-client (FE side) and TanStack's `useNavigate` (router).
// `useNavigate` is the only router hook TemplateSection needs.

const mockListTemplates = vi.fn()
const mockCreatePageFromTemplate = vi.fn()
const mockNavigate = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    listTemplates: (...args: unknown[]) => mockListTemplates(...args),
    createPageFromTemplate: (...args: unknown[]) =>
      mockCreatePageFromTemplate(...args),
  },
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

// ── Helpers ────────────────────────────────────────────────────────

const TEMPLATE_MEETING = {
  name: 'meeting-notes',
  full_name: 'template/meeting-notes',
  block_count: 5,
  card_shape: 'reference',
  icon: null,
  cssclass: null,
}

const TEMPLATE_REFERENCE = {
  name: 'reference',
  full_name: 'template/reference',
  block_count: 1,
  card_shape: 'reference',
  icon: null,
  cssclass: null,
}

beforeEach(() => {
  mockListTemplates.mockReset()
  mockCreatePageFromTemplate.mockReset()
  mockNavigate.mockReset()
})

// ── Tests ──────────────────────────────────────────────────────────

describe('TemplateSection — sidebar-template-ux', () => {
  it('renders the "Plantillas" group header (DESIGN.md §9.1, D5)', async () => {
    mockListTemplates.mockResolvedValue([TEMPLATE_MEETING])

    render(<TemplateSection />)

    // Header is always present (even while loading) per spec scenario
    // "Empty template list" — the header never disappears for a
    // successfully-rendered TemplateSection.
    const heading = screen.getByRole('heading', { name: 'Plantillas' })
    expect(heading).toBeInTheDocument()

    // Flush the resolved mock so the act() tracker doesn't fire after
    // the test exits.
    await waitFor(() => {
      expect(
        screen.getByTestId('template-item-meeting-notes'),
      ).toBeInTheDocument()
    })
  })

  it('renders one item per template returned by api.listTemplates (spec: Templates render when expanded)', async () => {
    mockListTemplates.mockResolvedValue([
      TEMPLATE_MEETING,
      TEMPLATE_REFERENCE,
    ])

    render(<TemplateSection />)

    await waitFor(() => {
      expect(
        screen.getByTestId('template-item-meeting-notes'),
      ).toBeInTheDocument()
    })
    expect(
      screen.getByTestId('template-item-reference'),
    ).toBeInTheDocument()
    expect(screen.getByText('meeting-notes')).toBeInTheDocument()
    expect(screen.getByText('reference')).toBeInTheDocument()
  })

  it('renders the empty-state message when api.listTemplates returns [] (spec: Empty template list)', async () => {
    mockListTemplates.mockResolvedValue([])

    render(<TemplateSection />)

    // Header is still rendered.
    expect(
      screen.getByRole('heading', { name: 'Plantillas' }),
    ).toBeInTheDocument()

    // Empty-state message appears.
    await waitFor(() => {
      expect(screen.getByText(/no templates available/i)).toBeInTheDocument()
    })
  })

  it('shows a loading skeleton while api.listTemplates is pending (spec: list render lifecycle)', async () => {
    // Never-resolving promise keeps the section in the loading state.
    mockListTemplates.mockReturnValue(new Promise(() => {}))

    render(<TemplateSection />)

    // Skeleton present (the same SidebarSkeleton we use elsewhere) is
    // recognised by the data-testid used by all sidebar sections.
    expect(screen.getByTestId('sidebar-skeleton')).toBeInTheDocument()
    expect(
      screen.queryByTestId('template-item-meeting-notes'),
    ).not.toBeInTheDocument()
  })

  it('clicking a template calls api.createPageFromTemplate with `-1` collision suffix (spec: Click creates and navigates)', async () => {
    mockListTemplates.mockResolvedValue([TEMPLATE_MEETING])
    mockCreatePageFromTemplate.mockResolvedValue({
      page: {
        id: 'p1',
        name: 'meeting-notes-1',
        title: 'meeting-notes-1',
        journal: false,
        journalDay: null,
        createdAt: '',
      },
      blocksCreated: 5,
    })

    const user = userEvent.setup()
    render(<TemplateSection />)

    await waitFor(() => {
      expect(
        screen.getByTestId('template-item-meeting-notes'),
      ).toBeInTheDocument()
    })

    await user.click(screen.getByTestId('template-item-meeting-notes'))

    // The server resolves templates by their full name (e.g.
    // `template/meeting-notes`); passing the short name produces a 404 in
    // production. The pageName keeps the user-facing `-1` collision suffix.
    await waitFor(() => {
      expect(mockCreatePageFromTemplate).toHaveBeenCalledWith({
        templateName: 'template/meeting-notes',
        pageName: 'meeting-notes-1',
      })
    })
  })

  it('passes template.full_name to createPageFromTemplate, not template.name (regression — C1 from sdd-verify)', async () => {
    // Regression test for finding C1 from the sdd-verify phase.
    // Production was passing `template.name` (e.g. "short") to
    // `api.createPageFromTemplate`; the server requires the full name
    // (`template/short`) per `CreatePageFromTemplateRequest` (api.ts:100).
    // Mocks don't validate argument shape, so the bug only surfaced against
    // the real server. This test pins the contract.
    const TEMPLATE_SHORT = {
      name: 'short',
      full_name: 'template/short',
      block_count: 1,
      card_shape: 'reference' as const,
      icon: null,
      cssclass: null,
    }
    mockListTemplates.mockResolvedValue([TEMPLATE_SHORT])
    mockCreatePageFromTemplate.mockResolvedValue({
      page: {
        id: 'p-short',
        name: 'short-1',
        title: 'short-1',
        journal: false,
        journalDay: null,
        createdAt: '',
      },
      blocksCreated: 1,
    })

    const user = userEvent.setup()
    render(<TemplateSection />)

    await waitFor(() => {
      expect(screen.getByTestId('template-item-short')).toBeInTheDocument()
    })

    await user.click(screen.getByTestId('template-item-short'))

    // Must pass the full name (server contract)…
    await waitFor(() => {
      expect(mockCreatePageFromTemplate).toHaveBeenCalledWith(
        expect.objectContaining({ templateName: 'template/short' }),
      )
    })
    // …and must NOT pass the short name (production would 404).
    expect(mockCreatePageFromTemplate).not.toHaveBeenCalledWith(
      expect.objectContaining({ templateName: 'short' }),
    )
    // `pageName` retains the user-facing `-1` collision suffix and is built
    // from the SHORT name — it is the display identifier, not the lookup key.
    expect(mockCreatePageFromTemplate).toHaveBeenCalledWith(
      expect.objectContaining({ pageName: 'short-1' }),
    )
  })

  it('clicking a template navigates to /page/<created-name> (spec: Click creates and navigates)', async () => {
    mockListTemplates.mockResolvedValue([TEMPLATE_MEETING])
    mockCreatePageFromTemplate.mockResolvedValue({
      page: {
        id: 'p1',
        name: 'meeting-notes-1',
        title: 'meeting-notes-1',
        journal: false,
        journalDay: null,
        createdAt: '',
      },
      blocksCreated: 5,
    })

    const user = userEvent.setup()
    render(<TemplateSection />)

    await waitFor(() => {
      expect(
        screen.getByTestId('template-item-meeting-notes'),
      ).toBeInTheDocument()
    })

    await user.click(screen.getByTestId('template-item-meeting-notes'))

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/meeting-notes-1',
      })
    })
  })

  it('renders an aria-label "Create page from template: <name>" on each item (spec: Screen reader announcement, §Accessibility)', async () => {
    mockListTemplates.mockResolvedValue([TEMPLATE_MEETING, TEMPLATE_REFERENCE])

    render(<TemplateSection />)

    await waitFor(() => {
      expect(
        screen.getByTestId('template-item-meeting-notes'),
      ).toBeInTheDocument()
    })

    // The accessible name is on the button — querying by role gives a
    // direct handle on the trigger element.
    expect(
      screen.getByRole('button', {
        name: 'Create page from template: meeting-notes',
      }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', {
        name: 'Create page from template: reference',
      }),
    ).toBeInTheDocument()
  })

  it('ignores additional clicks while a create is in flight (spec: Click on busy item is ignored)', async () => {
    mockListTemplates.mockResolvedValue([TEMPLATE_MEETING])
    // Never-resolving create promise keeps the item in the busy state.
    mockCreatePageFromTemplate.mockReturnValue(new Promise(() => {}))

    const user = userEvent.setup()
    render(<TemplateSection />)

    await waitFor(() => {
      expect(
        screen.getByTestId('template-item-meeting-notes'),
      ).toBeInTheDocument()
    })

    const item = screen.getByTestId('template-item-meeting-notes')

    await user.click(item)
    // Second click is dropped — `disabled` is the contract.
    await user.click(item)

    // Even if the disabled attribute is checked manually (some
    // implementations use aria-busy), the API was only called once.
    await waitFor(() => {
      expect(mockCreatePageFromTemplate).toHaveBeenCalledTimes(1)
    })
  })

  it('does not render anything when collapsed=true (spec: Templates hidden when collapsed)', () => {
    // Use a never-resolving mock so the effect's setState never fires —
    // otherwise the test would receive a noisy act() warning after the
    // assertions ran. The behavioural contract under test is the
    // IMMEDIATE render result, not the eventual state.
    mockListTemplates.mockReturnValue(new Promise(() => {}))

    const { container } = render(<TemplateSection collapsed={true} />)

    // The whole section is hidden — the GroupHeader returns null when
    // collapsed, so the section becomes an empty section with no header.
    expect(
      screen.queryByRole('heading', { name: 'Plantillas' }),
    ).not.toBeInTheDocument()
    expect(container.querySelector('section')).toBeNull()
  })

  it('shows a toast.error and renders the empty state when api.listTemplates rejects (spec: Fetch failure shows error)', async () => {
    const errorSpy = vi.spyOn(toast, 'error').mockImplementation(() => '')
    mockListTemplates.mockRejectedValue(new Error('network down'))

    render(<TemplateSection />)

    // Flush the rejection so the effect's catch handler runs inside
    // the act() boundary.
    await waitFor(() => {
      expect(errorSpy).toHaveBeenCalledWith(
        expect.stringMatching(/failed to load templates/i),
      )
    })

    // Empty state is shown — same string the success-empty case uses.
    expect(screen.getByText(/no templates available/i)).toBeInTheDocument()

    errorSpy.mockRestore()
  })
})
