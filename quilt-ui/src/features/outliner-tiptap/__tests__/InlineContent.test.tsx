import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { InlineContent } from '../InlineContent'

// ──── WASM bridge mock ──────────────────────────────────────────────
//
// InlineContent delegates inline parsing to the Rust WASM engine via
// `useWasm()`. We mock the provider module so we can control the
// returned segments deterministically — no real WASM, no flake.

const mockParseInline = vi.fn<(content: string) => { segments: any[] }>()

vi.mock('@core/wasm-bridge/WasmProvider', () => ({
  useWasm: () => ({
    loaded: true,
    error: null,
    wasmGetVersion: vi.fn(() => 'test'),
    wasmPing: vi.fn(() => true),
    wasmGetState: vi.fn(),
    wasmLoadPage: vi.fn(),
    wasmDispatch: vi.fn(),
    wasmUndo: vi.fn(),
    wasmRedo: vi.fn(),
    // InlineContent only calls wasmParseInline; the rest is just to
    // satisfy the WasmState shape.
    wasmParseInline: (content: string) => mockParseInline(content),
    retry: vi.fn(),
  }),
  ensureWasmLoaded: vi.fn().mockResolvedValue(undefined),
}))

// ──── API client mock ──────────────────────────────────────────────
//
// `handlePageRefClick` calls `api.createPage` when the target page
// doesn't exist in the pageMap. We mock that call so the test
// verifies the create-on-click behavior without hitting the network.

const mockCreatePage = vi.fn().mockResolvedValue({ id: 'new', name: 'NewPage' })
const mockListPages = vi.fn().mockResolvedValue([])

vi.mock('@core/api-client', () => ({
  api: {
    createPage: (...args: unknown[]) => mockCreatePage(...args),
    listPages: (...args: unknown[]) => mockListPages(...args),
  },
}))

beforeEach(() => {
  mockParseInline.mockReset()
  mockCreatePage.mockClear()
  mockListPages.mockClear()
})

describe('InlineContent', () => {
  it('renders bold text as <strong>', () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'bold', value: 'hello' }],
    })
    render(<InlineContent content="**hello**" />)
    const el = screen.getByText('hello')
    expect(el.tagName).toBe('STRONG')
  })

  it('renders italic text as <em>', () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'italic', value: 'world' }],
    })
    render(<InlineContent content="*world*" />)
    const el = screen.getByText('world')
    expect(el.tagName).toBe('EM')
  })

  it('renders page ref as link with /page/ href', () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'MyPage' }],
    })
    render(<InlineContent content="[[MyPage]]" />)
    const link = screen.getByText('MyPage')
    expect(link.tagName).toBe('A')
    expect(link).toHaveAttribute('href', '/page/MyPage')
  })

  it('renders code segment as <code>', () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'code', value: 'const x = 1' }],
    })
    render(<InlineContent content="`const x = 1`" />)
    const el = screen.getByText('const x = 1')
    expect(el.tagName).toBe('CODE')
  })

  it('renders the real Rust serde enum shape for bold segments', () => {
    mockParseInline.mockReturnValue({
      segments: [
        {
          Bold: {
            content: 'hello',
            raw: '**hello**',
            range: { start: 0, end: 9 },
          },
        },
      ],
    })
    render(<InlineContent content="**hello**" />)
    const el = screen.getByText('hello')
    expect(el.tagName).toBe('STRONG')
  })

  it('renders the real Rust serde enum shape for page refs', () => {
    mockParseInline.mockReturnValue({
      segments: [
        {
          PageRef: {
            page_name: 'MyPage',
            raw: '[[MyPage]]',
            range: { start: 0, end: 10 },
          },
        },
      ],
    })
    render(<InlineContent content="[[MyPage]]" />)
    const link = screen.getByText('MyPage')
    expect(link.tagName).toBe('A')
    expect(link).toHaveAttribute('href', '/page/MyPage')
  })

  it('renders the real Rust serde enum shape for headers', () => {
    mockParseInline.mockReturnValue({
      segments: [
        {
          Header: {
            level: 1,
            content: 'Hola',
            raw: '# Hola',
            range: { start: 0, end: 6 },
          },
        },
      ],
    })
    render(<InlineContent content="# Hola" />)
    const el = screen.getByText('Hola')
    expect(el.tagName).toBe('SPAN')
    expect(el).toHaveStyle({ fontWeight: '700' })
  })

  it('renders status property as a pill badge (uppercase value)', () => {
    mockParseInline.mockReturnValue({
      segments: [
        { type: 'property', value: { key: 'status', value: 'todo' } },
      ],
    })
    render(<InlineContent content="status:: todo" />)
    // The renderer uppercases the value ("todo" → "TODO").
    expect(screen.getByText('TODO')).toBeInTheDocument()
  })

  it('renders priority property as a "P<value>" badge', () => {
    mockParseInline.mockReturnValue({
      segments: [
        { type: 'property', value: { key: 'priority', value: 'A' } },
      ],
    })
    render(<InlineContent content="priority:: A" />)
    // Renderer emits "P" + value.toUpperCase() = "PA".
    expect(screen.getByText('PA')).toBeInTheDocument()
  })

  it('renders a missing page ref with dimmed style', () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'NonExistent' }],
    })
    // Pass an empty pageMap so the page is treated as missing.
    render(<InlineContent content="[[NonExistent]]" pageMap={new Map()} />)
    const link = screen.getByText('NonExistent')
    // Renderer applies opacity: 0.6 when page is not in the map.
    expect(link).toHaveStyle({ opacity: '0.6' })
  })

  it('renders an existing page ref at full opacity', () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'Foo' }],
    })
    const pageMap = new Map([['Foo', { id: '1', name: 'Foo', title: null } as any]])
    render(<InlineContent content="[[Foo]]" pageMap={pageMap} />)
    const link = screen.getByText('Foo')
    expect(link).toHaveStyle({ opacity: '1' })
  })

  it('renders mixed segments in order', () => {
    mockParseInline.mockReturnValue({
      segments: [
        { type: 'text', value: 'This is ' },
        { type: 'bold', value: 'important' },
        { type: 'text', value: ' text' },
      ],
    })
    render(<InlineContent content="This is **important** text" />)
    expect(screen.getByText('This is')).toBeInTheDocument()
    expect(screen.getByText('important').tagName).toBe('STRONG')
    expect(screen.getByText('text')).toBeInTheDocument()
  })

  it('falls back to raw content when the parser returns no segments', () => {
    mockParseInline.mockReturnValue({ segments: [] })
    render(<InlineContent content="plain text" />)
    expect(screen.getByText('plain text')).toBeInTheDocument()
  })

  it('falls back to raw content when the parser throws', () => {
    mockParseInline.mockImplementation(() => {
      throw new Error('WASM exploded')
    })
    render(<InlineContent content="oops" />)
    expect(screen.getByText('oops')).toBeInTheDocument()
  })

  it('falls back to raw content when the parser returns an unknown shape', () => {
    mockParseInline.mockReturnValue({
      segments: [{ nope: { weird: true } }],
    })
    render(<InlineContent content="plain fallback" />)
    expect(screen.getByText('plain fallback')).toBeInTheDocument()
  })

  // ──── Create-on-click (Logseq parity) ──────────────────────────
  //
  // G2 from the wikilinks audit: clicking a [[Page]] link for a
  // page that doesn't exist should create the page on the fly
  // (rather than navigating to a 404).

  it('creates the page on click when target is not in pageMap', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'NewPage' }],
    })
    // Empty pageMap → page does NOT exist
    render(
      <InlineContent
        content="[[NewPage]]"
        pageMap={new Map()}
      />,
    )
    const link = screen.getByText('NewPage')
    fireEvent.click(link)

    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'NewPage' })
    })
  })

  it('does NOT create the page when target already exists in pageMap', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'ExistingPage' }],
    })
    const pageMap = new Map([
      ['ExistingPage', { id: '1', name: 'ExistingPage', title: null } as any],
    ])
    render(
      <InlineContent
        content="[[ExistingPage]]"
        pageMap={pageMap}
      />,
    )
    const link = screen.getByText('ExistingPage')
    fireEvent.click(link)

    // Wait a tick for any spurious async call
    await new Promise(r => setTimeout(r, 10))
    expect(mockCreatePage).not.toHaveBeenCalled()
  })

  it('still navigates even when createPage throws (concurrent create, etc.)', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'RacePage' }],
    })
    mockCreatePage.mockRejectedValueOnce(new Error('UNIQUE constraint failed'))

    render(
      <InlineContent
        content="[[RacePage]]"
        pageMap={new Map()}
      />,
    )
    const link = screen.getByText('RacePage')

    // Should not throw — the click handler catches the rejection so the
    // user doesn't see a broken UI; navigation still happens.
    expect(() => fireEvent.click(link)).not.toThrow()

    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'RacePage' })
    })
  })

  // ──── G1: [[Page|alias]] rendering ─────────────────────────────────
  //
  // The alias is the display text only; the href and the page lookup
  // must always point at the page name. These tests pin down that
  // contract for both the simplified and the real Rust serde shapes.

  it('renders the alias text and href still points at the page name', () => {
    mockParseInline.mockReturnValue({
      segments: [
        { type: 'pageRef', value: { pageName: 'MyPage', alias: 'My Alias' } },
      ],
    })
    render(<InlineContent content="[[MyPage|My Alias]]" />)

    // The visible link text is the alias, not the page name.
    const link = screen.getByText('My Alias')
    expect(link.tagName).toBe('A')
    // …and the page name is NOT rendered as a separate text node.
    expect(screen.queryByText('MyPage')).not.toBeInTheDocument()
    // The href still points at the page name (URL-encoded).
    expect(link).toHaveAttribute('href', '/page/MyPage')
  })

  it('renders the page name when no alias is present (backward compat)', () => {
    mockParseInline.mockReturnValue({
      segments: [
        { type: 'pageRef', value: { pageName: 'MyPage', alias: null } },
      ],
    })
    render(<InlineContent content="[[MyPage]]" />)

    const link = screen.getByText('MyPage')
    expect(link.tagName).toBe('A')
    expect(link).toHaveAttribute('href', '/page/MyPage')
  })

  it('renders the alias using the real Rust serde enum shape', () => {
    mockParseInline.mockReturnValue({
      segments: [
        {
          PageRef: {
            page_name: 'Real Page',
            alias: 'display alias',
            raw: '[[Real Page|display alias]]',
            range: { start: 0, end: 28 },
          },
        },
      ],
    })
    render(<InlineContent content="[[Real Page|display alias]]" />)

    // Display text is the alias…
    const link = screen.getByText('display alias')
    expect(link.tagName).toBe('A')
    // …and the href points to the actual page name (URL-encoded).
    expect(link).toHaveAttribute('href', '/page/Real%20Page')
    // The page name itself is not visible.
    expect(screen.queryByText('Real Page')).not.toBeInTheDocument()
  })

  it('uses the page name (not the alias) for create-on-click', async () => {
    mockParseInline.mockReturnValue({
      segments: [
        { type: 'pageRef', value: { pageName: 'Target', alias: 'pretty' } },
      ],
    })
    // Empty pageMap → Target does not exist; click should create it.
    render(
      <InlineContent content="[[Target|pretty]]" pageMap={new Map()} />,
    )
    const link = screen.getByText('pretty')
    fireEvent.click(link)

    // The API call must use the PAGE NAME, not the display alias.
    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'Target' })
    })
  })
})
