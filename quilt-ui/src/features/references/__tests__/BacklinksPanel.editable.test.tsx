import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { BacklinksPanel } from '../BacklinksPanel'

const mockGetPageBacklinks = vi.fn()
const mockUpdateReferenceContext = vi.fn()
const mockNavigate = vi.fn()
const mockWriteText = vi.fn().mockResolvedValue(undefined)

vi.mock('@core/api-client', () => ({
  api: {
    getPageBacklinks: (name: string) => mockGetPageBacklinks(name),
    updateReferenceContext: (params: {
      sourceBlockId: string
      targetPageName: string
      context: string | null
    }) => mockUpdateReferenceContext(params),
  },
  // Keep the cache invalidator as a no-op in tests; we only care that
  // the api-client surface is the one we exercise.
  sessionCache: {
    invalidateAll: vi.fn(),
    invalidatePage: vi.fn(),
  },
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

vi.mock('react-hot-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
  },
}))

function makeBacklink(overrides: Partial<{
  sourceBlockId: string
  sourcePageName: string
  contentPreview: string
  context: string
}> = {}) {
  return {
    sourceBlockId: 'block-1',
    sourcePageName: 'source-page',
    contentPreview: 'Original block content for block 1',
    context: 'Original block content for block 1',
    ...overrides,
  }
}

describe('BacklinksPanel — Q028: Editable Backlinks', () => {
  beforeEach(() => {
    mockGetPageBacklinks.mockReset()
    mockUpdateReferenceContext.mockReset()
    mockNavigate.mockReset()
    mockWriteText.mockReset()
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: mockWriteText },
    })
    // Default mock: PUT returns the updated DTO
    mockUpdateReferenceContext.mockImplementation(
      async (params: { sourceBlockId: string; context: string | null }) => ({
        sourceBlockId: params.sourceBlockId,
        sourcePageName: 'source-page',
        contentPreview: 'Original block content for block 1',
        context: params.context ?? 'Original block content for block 1',
      }),
    )
  })

  // ── 1. BacklinksPanel shows context text ───────────────────────

  it('renders the `context` snippet for each backlink (not the raw content preview)', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue([
      makeBacklink({
        sourceBlockId: 'block-1',
        contentPreview: 'RAW PREVIEW',
        context: 'A meaningful custom snippet the user set',
      }),
      makeBacklink({
        sourceBlockId: 'block-2',
        contentPreview: 'another raw preview',
        context: 'another raw preview', // no override — falls back
      }),
    ])

    render(<BacklinksPanel pageName="demo" isOpen={true} />)

    // Expand the panel
    await user.click(screen.getByTestId('backlinks-panel-header'))

    // First backlink shows the custom snippet, NOT the raw preview
    expect(
      screen.getByText('A meaningful custom snippet the user set'),
    ).toBeInTheDocument()
    expect(screen.queryByText('RAW PREVIEW')).not.toBeInTheDocument()

    // Second backlink shows the default snippet (no override set)
    expect(screen.getByText('another raw preview')).toBeInTheDocument()
  })

  // ── 2. Edit button appears on hover for each backlink ──────────
  //
  // We use a `data-testid` on the row so the test can target it
  // precisely. The button itself is `aria-label` titled
  // "Edit context" and lives inside the row.

  it('shows an Edit context button on hover for each backlink', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue([makeBacklink()])

    render(<BacklinksPanel pageName="demo" isOpen={true} />)
    await user.click(screen.getByTestId('backlinks-panel-header'))

    const editButton = await screen.findByRole('button', { name: /edit context/i })
    expect(editButton).toBeInTheDocument()
  })

  // ── 3. Click → inline edit mode activates ─────────────────────

  it('switches to inline edit mode when the Edit context button is clicked', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue([
      makeBacklink({
        sourceBlockId: 'block-x',
        context: 'Old context',
      }),
    ])

    render(<BacklinksPanel pageName="demo" isOpen={true} />)
    await user.click(screen.getByTestId('backlinks-panel-header'))

    // Before edit: a paragraph with the old context is shown
    expect(screen.getByText('Old context')).toBeInTheDocument()

    // Click edit
    const editButton = screen.getByRole('button', { name: /edit context/i })
    await user.click(editButton)

    // After edit: a textarea/input appears, pre-filled with the
    // current context, AND the navigation click on the row must be
    // suppressed (we don't want clicking inside the input to
    // navigate away).
    const textarea = screen.getByLabelText('Edit context') as HTMLTextAreaElement
    expect(textarea).toBeInTheDocument()
    expect(textarea.value).toBe('Old context')

    // Save and cancel buttons appear
    expect(screen.getByRole('button', { name: /save/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument()
  })

  // ── 4. Save calls API correctly ────────────────────────────────

  it('calls the API with the new context when Save is clicked', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue([
      makeBacklink({
        sourceBlockId: 'block-1',
        sourcePageName: 'source-1',
        context: 'Old text',
      }),
    ])

    render(<BacklinksPanel pageName="target-page" isOpen={true} />)
    await user.click(screen.getByTestId('backlinks-panel-header'))

    // Enter edit mode
    await user.click(screen.getByRole('button', { name: /edit context/i }))

    // Change the text
    const textarea = screen.getByLabelText('Edit context') as HTMLTextAreaElement
    await user.clear(textarea)
    await user.type(textarea, 'New snippet for the user')

    // Save
    await user.click(screen.getByRole('button', { name: /save/i }))

    await waitFor(() =>
      expect(mockUpdateReferenceContext).toHaveBeenCalledWith({
        sourceBlockId: 'block-1',
        targetPageName: 'target-page',
        context: 'New snippet for the user',
      }),
    )
  })

  it('passes null to the API when the user clears the context and saves', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue([
      makeBacklink({
        sourceBlockId: 'block-1',
        context: 'Old text',
      }),
    ])

    render(<BacklinksPanel pageName="target-page" isOpen={true} />)
    await user.click(screen.getByTestId('backlinks-panel-header'))
    await user.click(screen.getByRole('button', { name: /edit context/i }))

    const textarea = screen.getByLabelText('Edit context') as HTMLTextAreaElement
    await user.clear(textarea)

    await user.click(screen.getByRole('button', { name: /save/i }))

    await waitFor(() =>
      expect(mockUpdateReferenceContext).toHaveBeenCalledWith({
        sourceBlockId: 'block-1',
        targetPageName: 'target-page',
        context: null,
      }),
    )
  })

  // ── 5. Cancel reverts changes ─────────────────────────────────

  it('does NOT call the API when Cancel is clicked and reverts to the original context', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue([
      makeBacklink({
        sourceBlockId: 'block-1',
        context: 'Original text',
      }),
    ])

    render(<BacklinksPanel pageName="target-page" isOpen={true} />)
    await user.click(screen.getByTestId('backlinks-panel-header'))

    // Edit and type
    await user.click(screen.getByRole('button', { name: /edit context/i }))
    const textarea = screen.getByLabelText('Edit context') as HTMLTextAreaElement
    await user.clear(textarea)
    await user.type(textarea, 'Typing-then-cancelling')

    // Cancel
    await user.click(screen.getByRole('button', { name: /cancel/i }))

    // API was not called
    expect(mockUpdateReferenceContext).not.toHaveBeenCalled()

    // The original context is back on screen
    expect(screen.getByText('Original text')).toBeInTheDocument()
    expect(screen.queryByText('Typing-then-cancelling')).not.toBeInTheDocument()

    // The textarea (the inline editor) is gone — the row is back in
    // read-only mode. The Edit button stays on the row, so we check
    // for the absence of the textarea specifically.
    expect(screen.queryByRole('textbox', { name: /edit context/i })).not.toBeInTheDocument()
  })

  // ── 6. Navigation is suppressed while editing ─────────────────

  it('does NOT navigate when the user clicks inside the editor (only on the row body)', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue([
      makeBacklink({
        sourceBlockId: 'block-1',
        sourcePageName: 'source-page-1',
        context: 'snippet',
      }),
    ])

    render(<BacklinksPanel pageName="target-page" isOpen={true} />)
    await user.click(screen.getByTestId('backlinks-panel-header'))

    // Enter edit mode
    await user.click(screen.getByRole('button', { name: /edit context/i }))

    // Click the textarea (focus it) — must not navigate
    const textarea = screen.getByLabelText('Edit context') as HTMLTextAreaElement
    await user.click(textarea)
    expect(mockNavigate).not.toHaveBeenCalled()
  })
})
