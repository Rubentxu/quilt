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

// ──── TanStack Router mock ─────────────────────────────────────────
//
// InlineContent uses `useNavigate` to navigate when the user clicks
// a [[Page]] or #tag link. The previous implementation relied on
// `window.location.hash`, which silently failed with the browser
// history used by TanStack Router — see the bug this commit fixes.
// We mock `useNavigate` so tests can assert the navigation call.

const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

beforeEach(() => {
  mockParseInline.mockReset()
  mockCreatePage.mockClear()
  mockListPages.mockClear()
  mockNavigate.mockClear()
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
      segments: [{ type: 'pageRef', value: 'mypage' }],
    })
    render(<InlineContent content="[[mypage]]" />)
    const link = screen.getByText('mypage')
    expect(link.tagName).toBe('A')
    // href is the canonical (lowercase) form so Cmd/Ctrl-click opens
    // the same URL the JS click would navigate to.
    expect(link).toHaveAttribute('href', '/page/mypage')
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
            page_name: 'mypage',
            raw: '[[mypage]]',
            range: { start: 0, end: 10 },
          },
        },
      ],
    })
    render(<InlineContent content="[[mypage]]" />)
    const link = screen.getByText('mypage')
    expect(link.tagName).toBe('A')
    // href is the canonical form regardless of the page_name case.
    expect(link).toHaveAttribute('href', '/page/mypage')
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
      segments: [{ type: 'pageRef', value: 'foo' }],
    })
    const pageMap = new Map([['foo', { id: '1', name: 'foo', title: null } as any]])
    render(<InlineContent content="[[foo]]" pageMap={pageMap} />)
    const link = screen.getByText('foo')
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

  // ──── Create-on-click (Quilt parity) ──────────────────────────
  //
  // G2 from the wikilinks audit: clicking a [[Page]] link for a
  // page that doesn't exist should create the page on the fly
  // (rather than navigating to a 404).

  it('creates the page on click when target is not in pageMap', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'newpage' }],
    })
    // Empty pageMap → page does NOT exist
    render(
      <InlineContent
        content="[[newpage]]"
        pageMap={new Map()}
      />,
    )
    const link = screen.getByText('newpage')
    fireEvent.click(link)

    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'newpage' })
    })
  })

  it('does NOT create the page when target already exists in pageMap', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'existingpage' }],
    })
    // The pageMap keys are always lowercase (the server's canonical
    // form — see Page::normalize_name). The user-typed reference is
    // also lowercased before the lookup.
    const pageMap = new Map([
      ['existingpage', { id: '1', name: 'existingpage', title: null } as any],
    ])
    render(
      <InlineContent
        content="[[existingpage]]"
        pageMap={pageMap}
      />,
    )
    const link = screen.getByText('existingpage')
    fireEvent.click(link)

    // Wait a tick for any spurious async call
    await new Promise(r => setTimeout(r, 10))
    expect(mockCreatePage).not.toHaveBeenCalled()
  })

  it('still navigates even when createPage throws (concurrent create, etc.)', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'racepage' }],
    })
    mockCreatePage.mockRejectedValueOnce(new Error('UNIQUE constraint failed'))

    render(
      <InlineContent
        content="[[racepage]]"
        pageMap={new Map()}
      />,
    )
    const link = screen.getByText('racepage')

    // Should not throw — the click handler catches the rejection so the
    // user doesn't see a broken UI; navigation still happens.
    expect(() => fireEvent.click(link)).not.toThrow()

    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'racepage' })
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
        { type: 'pageRef', value: { pageName: 'mypage', alias: 'My Alias' } },
      ],
    })
    render(<InlineContent content="[[mypage|My Alias]]" />)

    // The visible link text is the alias, not the page name.
    const link = screen.getByText('My Alias')
    expect(link.tagName).toBe('A')
    // …and the page name is NOT rendered as a separate text node.
    expect(screen.queryByText('mypage')).not.toBeInTheDocument()
    // The href points at the canonical (lowercase) form.
    expect(link).toHaveAttribute('href', '/page/mypage')
  })

  it('renders the page name when no alias is present (backward compat)', () => {
    mockParseInline.mockReturnValue({
      segments: [
        { type: 'pageRef', value: { pageName: 'mypage', alias: null } },
      ],
    })
    render(<InlineContent content="[[mypage]]" />)

    const link = screen.getByText('mypage')
    expect(link.tagName).toBe('A')
    expect(link).toHaveAttribute('href', '/page/mypage')
  })

  it('renders the alias using the real Rust serde enum shape', () => {
    mockParseInline.mockReturnValue({
      segments: [
        {
          PageRef: {
            page_name: 'real page',
            alias: 'display alias',
            raw: '[[real page|display alias]]',
            range: { start: 0, end: 28 },
          },
        },
      ],
    })
    render(<InlineContent content="[[real page|display alias]]" />)

    // Display text is the alias…
    const link = screen.getByText('display alias')
    expect(link.tagName).toBe('A')
    // …and the href points to the canonical page name (URL-encoded).
    expect(link).toHaveAttribute('href', '/page/real%20page')
    // The page name itself is not visible.
    expect(screen.queryByText('real page')).not.toBeInTheDocument()
  })

  it('uses the page name (not the alias) for create-on-click', async () => {
    mockParseInline.mockReturnValue({
      segments: [
        { type: 'pageRef', value: { pageName: 'target', alias: 'pretty' } },
      ],
    })
    // Empty pageMap → target does not exist; click should create it.
    render(
      <InlineContent content="[[target|pretty]]" pageMap={new Map()} />,
    )
    const link = screen.getByText('pretty')
    fireEvent.click(link)

    // The API call must use the canonical PAGE NAME, not the display alias.
    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'target' })
    })
  })

  // Regression: clicking a [[wikilink]] must NOT bubble up to the parent
  // (BlockRow's onClick={handleStartEdit}), which would put the block in
  // edit mode instead of navigating to / creating the linked page.
  it('stops click propagation so BlockRow does not enter edit mode', async () => {
    mockParseInline.mockReturnValue({
      segments: [
        { type: 'pageRef', value: { pageName: 'linkedpage', alias: null } },
      ],
    })

    // Simulate BlockRow: an outer div with onClick that mimics
    // handleStartEdit. If propagation is correctly stopped, this
    // handler must NOT fire when the wikilink is clicked.
    const onParentClick = vi.fn()
    render(
      <div onClick={onParentClick} data-testid="block-row">
        <InlineContent content="[[linkedpage]]" pageMap={new Map()} />
      </div>,
    )

    const link = screen.getByText('linkedpage')
    fireEvent.click(link)

    // The wikilink's own handler should have run (page was missing →
    // createPage is called)…
    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'linkedpage' })
    })
    // …but the parent (BlockRow) must NOT have received the click.
    expect(onParentClick).not.toHaveBeenCalled()
  })

  // ──── Navigation: click on [[Page]] calls useNavigate ─────────────
  //
  // Regression: the old implementation did `window.location.hash = ...`,
  // which silently failed because TanStack Router uses
  // `createBrowserHistory` and never reads the URL hash. Pinning this
  // down with a test so the bug can't reappear.

  it('navigates via useNavigate when [[Page]] is clicked', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'somepage' }],
    })
    const pageMap = new Map([
      ['somepage', { id: '1', name: 'somepage', title: null } as any],
    ])
    render(<InlineContent content="[[somepage]]" pageMap={pageMap} />)

    fireEvent.click(screen.getByText('somepage'))

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/$name',
        params: { name: 'somepage' },
      })
    })
  })

  it('URL-encodes the page name when navigating to [[Page With Spaces]]', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'page with spaces' }],
    })
    const pageMap = new Map([
      ['page with spaces', { id: '1', name: 'page with spaces', title: null } as any],
    ])
    render(<InlineContent content="[[page with spaces]]" pageMap={pageMap} />)

    fireEvent.click(screen.getByText('page with spaces'))

    await waitFor(() => {
      // navigate is called with the canonical (lowercase) name; the
      // router URL-encodes internally. We just verify the params shape.
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/$name',
        params: { name: 'page with spaces' },
      })
    })
  })

  it('navigates even when createPage throws (concurrent create case)', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'racepage' }],
    })
    mockCreatePage.mockRejectedValueOnce(new Error('UNIQUE constraint failed'))

    render(<InlineContent content="[[racepage]]" pageMap={new Map()} />)

    expect(() => fireEvent.click(screen.getByText('racepage'))).not.toThrow()

    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'racepage' })
    })
    // And navigation must STILL happen, even though the create threw.
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/$name',
        params: { name: 'racepage' },
      })
    })
  })

  // ──── Navigation: #tag click ─────────────────────────────────────

  it('navigates to the tag page on #tag click (Quilt parity)', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'tag', value: 'project' }],
    })
    render(<InlineContent content="#project" pageMap={new Map()} />)

    fireEvent.click(screen.getByText('#project'))

    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'project' })
    })
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/$name',
        params: { name: 'project' },
      })
    })
  })

  it('does NOT create the tag page when it already exists in pageMap', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'tag', value: 'project' }],
    })
    const pageMap = new Map([
      ['project', { id: '1', name: 'project', title: null } as any],
    ])
    render(<InlineContent content="#project" pageMap={pageMap} />)

    fireEvent.click(screen.getByText('#project'))

    // No async create
    await new Promise(r => setTimeout(r, 10))
    expect(mockCreatePage).not.toHaveBeenCalled()
    // But navigation still happens
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/$name',
        params: { name: 'project' },
      })
    })
  })

  it('stops click propagation on #tag so BlockRow does not enter edit mode', () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'tag', value: 'topic' }],
    })
    const onParentClick = vi.fn()
    render(
      <div onClick={onParentClick}>
        <InlineContent content="#topic" pageMap={new Map()} />
      </div>,
    )

    fireEvent.click(screen.getByText('#topic'))
    expect(onParentClick).not.toHaveBeenCalled()
  })

  // ──── Navigation: ((block-id)) click ─────────────────────────────

  it('navigates to the parent page on ((block-id)) click', async () => {
    mockParseInline.mockReturnValue({
      segments: [
        { type: 'blockRef', value: 'block-uuid-abc' },
      ],
    })
    const blocks = [
      {
        id: 'block-uuid-abc',
        content: 'some block text',
        pageName: 'blockownerpage',
        pageId: 'p1',
      } as any,
    ]
    render(<InlineContent content="((block-uuid-abc))" blocks={blocks} />)

    fireEvent.click(screen.getByText('some block text'))

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/$name',
        params: { name: 'blockownerpage' },
      })
    })
  })

  it('does nothing when ((block-id)) is unresolved (no matching block)', () => {
    // blockId is in the segment but no block with that id exists in
    // `blocks`. The renderer should show the "(missing block)" placeholder
    // and clicking it must not navigate or throw.
    mockParseInline.mockReturnValue({
      segments: [{ type: 'blockRef', value: 'missing-id' }],
    })
    const onParentClick = vi.fn()
    render(
      <div onClick={onParentClick}>
        <InlineContent content="((missing-id))" blocks={[]} />
      </div>,
    )

    expect(screen.getByText('(missing block)')).toBeInTheDocument()
    fireEvent.click(screen.getByText('(missing block)'))
    expect(mockNavigate).not.toHaveBeenCalled()
    expect(onParentClick).not.toHaveBeenCalled()
  })

  // ──── Case insensitivity (Quilt + server convention) ─────────────
  //
  // The server normalises page names to lowercase on insert (see
  // Page::normalize_name in crates/quilt-domain/src/entities/page.rs).
  // If the frontend used the user-typed case for the pageMap lookup, the
  // createPage call, and the navigate URL, then a user-typed
  // `[[My Notes]]` would create `mynotes` on the server but navigate
  // to `/page/My Notes` which the server 404s.
  //
  // These tests pin the fix: every site that reads a page name from
  // block content normalises it before any I/O.

  it('lowercases the user-typed [[Page]] before the pageMap lookup', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'MyNotes' }],
    })
    // The pageMap is populated by the server (which always returns
    // lowercase). The user-typed 'MyNotes' must match 'mynotes'.
    const pageMap = new Map([
      ['mynotes', { id: '1', name: 'mynotes', title: null } as any],
    ])
    render(<InlineContent content="[[MyNotes]]" pageMap={pageMap} />)

    fireEvent.click(screen.getByText('MyNotes'))

    // No createPage call because the canonical form exists in pageMap.
    await new Promise(r => setTimeout(r, 10))
    expect(mockCreatePage).not.toHaveBeenCalled()
    // And the navigation target is the canonical (lowercase) form.
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/$name',
        params: { name: 'mynotes' },
      })
    })
  })

  it('creates the page using the canonical (lowercase) name on click', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'BrandNewPage' }],
    })
    render(<InlineContent content="[[BrandNewPage]]" pageMap={new Map()} />)

    fireEvent.click(screen.getByText('BrandNewPage'))

    // The createPage call uses the canonical form. If the frontend
    // sent 'BrandNewPage' instead, the server would store 'brandnewpage'
    // and the subsequent navigate to /page/BrandNewPage would 404.
    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'brandnewpage' })
    })
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/$name',
        params: { name: 'brandnewpage' },
      })
    })
  })

  it('sets the href to the canonical (lowercase) form', () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: 'MixedCasePage' }],
    })
    render(<InlineContent content="[[MixedCasePage]]" pageMap={new Map()} />)

    const link = screen.getByText('MixedCasePage')
    // Cmd/Ctrl-click opens in a new tab using the href. The href must
    // already be the canonical form or the new tab 404s.
    expect(link).toHaveAttribute('href', '/page/mixedcasepage')
  })

  it('trims surrounding whitespace before lookups and navigation', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'pageRef', value: '  spaced  ' }],
    })
    render(<InlineContent content="[[  spaced  ]]" pageMap={new Map()} />)

    fireEvent.click(screen.getByText('spaced'))

    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'spaced' })
    })
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/$name',
        params: { name: 'spaced' },
      })
    })
  })

  it('lowercases #tag clicks the same way', async () => {
    mockParseInline.mockReturnValue({
      segments: [{ type: 'tag', value: 'ProjectX' }],
    })
    render(<InlineContent content="#ProjectX" pageMap={new Map()} />)

    fireEvent.click(screen.getByText('#ProjectX'))

    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({ name: 'projectx' })
    })
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/$name',
        params: { name: 'projectx' },
      })
    })
  })

  it('does not navigate when the user-typed name is empty/whitespace only', () => {
    mockParseInline.mockReturnValue({
      // Hypothetical — parser would normally not emit this, but we
      // guard the handler anyway. The display text is the raw '   '
      // the parser emitted, but the handler's normalised name is ''
      // and the early return kicks in.
      segments: [{ type: 'pageRef', value: '   ' }],
    })
    render(<InlineContent content="[[   ]]" pageMap={new Map()} />)

    // Find the rendered <a> by its empty href (the canonical form
    // collapses to '' so the href is '/page/' with nothing after).
    // Clicking it must not throw, must not call createPage, and must
    // not navigate.
    const link = document.querySelector('a[href="/page/"]') as HTMLElement
    expect(link).not.toBeNull()
    expect(() => fireEvent.click(link)).not.toThrow()
    expect(mockCreatePage).not.toHaveBeenCalled()
    expect(mockNavigate).not.toHaveBeenCalled()
  })
})
