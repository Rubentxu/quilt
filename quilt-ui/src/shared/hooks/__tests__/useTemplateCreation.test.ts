/**
 * Tests for useTemplateCreation — the "New from Template" wizard hook
 * extracted from BlockRow (architecture review candidate #5).
 *
 * The hook owns the multi-step flow: list templates → prompt user for
 * template choice (auto-pick if only one) → call createPageFromTemplate
 * → navigate to the new page. We mock the API, react-router's
 * `useNavigate`, and react-hot-toast so the test can drive the full
 * flow without standing up the real client.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'

// ──── Mutable mock state (hoisted) ───────────────────────────────────
//
// `vi.mock` is hoisted above imports, so anything the factory closes
// over must live in a `vi.hoisted` block — otherwise it would be
// referenced before initialised and vitest would throw a TDZ error.

const {
  mockListTemplates,
  mockCreatePageFromTemplate,
  mockNavigate,
  mockToastError,
  mockToastSuccess,
} = vi.hoisted(() => ({
  mockListTemplates: vi.fn(),
  mockCreatePageFromTemplate: vi.fn(),
  mockNavigate: vi.fn(),
  mockToastError: vi.fn(),
  mockToastSuccess: vi.fn(),
}))

vi.mock('@core/api-client', () => ({
  api: {
    listTemplates: (...args: unknown[]) => mockListTemplates(...args),
    createPageFromTemplate: (...args: unknown[]) => mockCreatePageFromTemplate(...args),
  },
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

vi.mock('react-hot-toast', () => ({
  default: {
    error: (...args: unknown[]) => mockToastError(...args),
    success: (...args: unknown[]) => mockToastSuccess(...args),
  },
}))

import { useTemplateCreation } from '../useTemplateCreation'
import type { CreatePageFromTemplateResponse, Page, TemplateSummary } from '@shared/types/api'

// ──── Fixtures ──────────────────────────────────────────────────────

const template = (overrides: Partial<TemplateSummary> = {}): TemplateSummary => ({
  name: 'reference',
  full_name: 'template/reference',
  block_count: 5,
  card_shape: 'inline',
  icon: null,
  cssclass: null,
  ...overrides,
})

const pageFixture = (overrides: Partial<Page> = {}): Page => ({
  id: 'p1',
  name: 'new-page',
  title: 'New Page',
  journal: false,
  journalDay: null,
  createdAt: '2026-01-01',
  ...overrides,
})

const createResponse = (overrides: Partial<CreatePageFromTemplateResponse> = {}): CreatePageFromTemplateResponse => ({
  page: pageFixture(),
  blocksCreated: 5,
  ...overrides,
})

// ──── Lifecycle ─────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
  // Default: no templates, no creation. Each test overrides as needed.
  mockListTemplates.mockResolvedValue([])
  mockCreatePageFromTemplate.mockResolvedValue(createResponse())
  // jsdom returns `null` for window.prompt by default — make the
  // default "no input" explicit so tests that don't care about the
  // prompt see the cancel branch.
  vi.spyOn(window, 'prompt').mockReturnValue(null)
})

// ──── Tests ─────────────────────────────────────────────────────────

describe('useTemplateCreation', () => {
  it('fetches templates on mount and exposes them via state', async () => {
    const tpls = [template({ name: 'reference' }), template({ name: 'daily' })]
    mockListTemplates.mockResolvedValue(tpls)

    const { result } = renderHook(() => useTemplateCreation())

    // Loading flag flips on while the promise is in flight, then off.
    expect(result.current.loading).toBe(true)
    expect(result.current.templates).toEqual([])

    await act(async () => {
      await Promise.resolve()
    })

    expect(mockListTemplates).toHaveBeenCalledTimes(1)
    expect(result.current.loading).toBe(false)
    expect(result.current.templates).toEqual(tpls)
  })

  it('createFromTemplate returns cancelled when pageName is empty', async () => {
    mockListTemplates.mockResolvedValue([template()])
    const onRestore = vi.fn()

    const { result } = renderHook(() => useTemplateCreation({ onRestore }))

    await act(async () => {
      await Promise.resolve()
    })

    let res: Awaited<ReturnType<typeof result.current.createFromTemplate>> | undefined
    await act(async () => {
      res = await result.current.createFromTemplate('   ', '/original')
    })

    expect(res).toEqual({ ok: false, reason: 'cancelled', error: 'Page name is required' })
    expect(onRestore).toHaveBeenCalledWith('/original')
    expect(mockCreatePageFromTemplate).not.toHaveBeenCalled()
  })

  it('createFromTemplate auto-selects the only template without prompting', async () => {
    const tpl = template({ name: 'reference', full_name: 'template/reference' })
    mockListTemplates.mockResolvedValue([tpl])
    mockCreatePageFromTemplate.mockResolvedValue(
      createResponse({ blocksCreated: 7 }),
    )
    const promptSpy = vi.spyOn(window, 'prompt')

    const { result } = renderHook(() => useTemplateCreation())

    await act(async () => {
      await Promise.resolve()
    })

    let res: Awaited<ReturnType<typeof result.current.createFromTemplate>> | undefined
    await act(async () => {
      res = await result.current.createFromTemplate('My Page', '/orig')
    })

    // No second prompt — only the page-name prompt would have fired,
    // and even that is bypassed because we pass pageName in directly.
    expect(promptSpy).not.toHaveBeenCalled()
    expect(mockCreatePageFromTemplate).toHaveBeenCalledWith({
      templateName: 'template/reference',
      pageName: 'My Page',
      title: 'My Page',
    })
    expect(res).toEqual({
      ok: true,
      page: pageFixture(),
      blocksCreated: 7,
    })
    expect(mockToastSuccess).toHaveBeenCalledWith(
      'Created from template "reference" (7 blocks)',
    )
    expect(mockNavigate).toHaveBeenCalledWith({
      to: '/page/$name',
      params: { name: 'new-page' },
    })
  })

  it('createFromTemplate prompts the user when multiple templates exist', async () => {
    const tpls = [
      template({ name: 'reference', full_name: 'template/reference' }),
      template({ name: 'daily', full_name: 'template/daily' }),
    ]
    mockListTemplates.mockResolvedValue(tpls)
    // First prompt would be pageName (skipped — passed in), second is
    // template choice. Simulate the user picking "daily".
    vi.spyOn(window, 'prompt')
      .mockReturnValueOnce('daily') // template choice
    mockCreatePageFromTemplate.mockResolvedValue(createResponse())

    const { result } = renderHook(() => useTemplateCreation())

    await act(async () => {
      await Promise.resolve()
    })

    let res: Awaited<ReturnType<typeof result.current.createFromTemplate>> | undefined
    await act(async () => {
      res = await result.current.createFromTemplate('My Page', '/orig')
    })

    expect(window.prompt).toHaveBeenCalledTimes(1)
    const [msg, defaultValue] = (window.prompt as unknown as ReturnType<typeof vi.fn>).mock.calls[0]
    expect(msg).toContain('reference')
    expect(msg).toContain('daily')
    expect(defaultValue).toBe('reference')
    expect(mockCreatePageFromTemplate).toHaveBeenCalledWith({
      templateName: 'template/daily',
      pageName: 'My Page',
      title: 'My Page',
    })
    expect(res?.ok).toBe(true)
  })

  it('createFromTemplate returns template_not_found when user picks an unknown template', async () => {
    mockListTemplates.mockResolvedValue([
      template({ name: 'reference' }),
      template({ name: 'daily' }),
    ])
    vi.spyOn(window, 'prompt').mockReturnValue('not-a-template')
    const onRestore = vi.fn()

    const { result } = renderHook(() => useTemplateCreation({ onRestore }))

    await act(async () => {
      await Promise.resolve()
    })

    let res: Awaited<ReturnType<typeof result.current.createFromTemplate>> | undefined
    await act(async () => {
      res = await result.current.createFromTemplate('My Page', '/orig')
    })

    expect(res).toEqual({
      ok: false,
      reason: 'template_not_found',
      error: 'Template not found: not-a-template',
    })
    expect(mockToastError).toHaveBeenCalledWith('Template not found: not-a-template')
    expect(onRestore).toHaveBeenCalledWith('/orig')
    expect(mockCreatePageFromTemplate).not.toHaveBeenCalled()
  })

  it('createFromTemplate returns no_templates when the list is empty', async () => {
    mockListTemplates.mockResolvedValue([])
    const onRestore = vi.fn()

    const { result } = renderHook(() => useTemplateCreation({ onRestore }))

    await act(async () => {
      await Promise.resolve()
    })

    let res: Awaited<ReturnType<typeof result.current.createFromTemplate>> | undefined
    await act(async () => {
      res = await result.current.createFromTemplate('My Page', '/orig')
    })

    expect(res).toEqual({
      ok: false,
      reason: 'no_templates',
      error: 'No templates found. Create one in the Plantillas section first.',
    })
    expect(mockToastError).toHaveBeenCalled()
    expect(onRestore).toHaveBeenCalledWith('/orig')
    expect(mockCreatePageFromTemplate).not.toHaveBeenCalled()
  })

  it('createFromTemplate returns api_error when createPageFromTemplate throws', async () => {
    mockListTemplates.mockResolvedValue([template()])
    mockCreatePageFromTemplate.mockRejectedValue(new Error('boom'))
    const onRestore = vi.fn()

    const { result } = renderHook(() => useTemplateCreation({ onRestore }))

    await act(async () => {
      await Promise.resolve()
    })

    let res: Awaited<ReturnType<typeof result.current.createFromTemplate>> | undefined
    await act(async () => {
      res = await result.current.createFromTemplate('My Page', '/orig')
    })

    expect(res).toEqual({
      ok: false,
      reason: 'api_error',
      error: 'Failed to create from template: boom',
    })
    expect(mockToastError).toHaveBeenCalledWith('Failed to create from template: boom')
    expect(onRestore).toHaveBeenCalledWith('/orig')
    expect(mockNavigate).not.toHaveBeenCalled()
  })
})
