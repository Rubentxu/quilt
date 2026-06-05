import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { useState } from 'react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { BlockRow, findNearestLink } from '../BlockRow'

// Place the caret at character offset `offset` inside a contentEditable.
// BlockRow's `handleInput` reads `window.getSelection()` to compute the
// text-before-cursor; in jsdom the selection is undefined after a plain
// `textContent` write, so we set it explicitly.
function setCaretPosition(el: HTMLElement, offset: number) {
  el.focus()
  const range = document.createRange()
  const textNode = el.firstChild
  if (textNode && textNode.nodeType === Node.TEXT_NODE) {
    const safe = Math.min(offset, textNode.textContent?.length ?? 0)
    range.setStart(textNode, safe)
    range.collapse(true)
  } else {
    range.selectNodeContents(el)
    range.collapse(false)
  }
  const sel = window.getSelection()
  sel?.removeAllRanges()
  sel?.addRange(range)
}

// Click the read-mode div to enter edit mode. Using `getByText` is
// unreliable when the block content is empty (matches every empty
// text node in the tree); the read mode has class `block-content-read`.
function clickToEdit() {
  const read = document.querySelector('.block-content-read') as HTMLElement | null
  expect(read).not.toBeNull()
  fireEvent.click(read!)
}

const mockUpdateBlock = vi.fn()
const mockListPages = vi.fn().mockResolvedValue([])
const mockListTemplates = vi.fn().mockResolvedValue([])
const mockCreatePageFromTemplate = vi.fn()
const mockWriteText = vi.fn().mockResolvedValue(undefined)
const mockParseInline = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    updateBlock: (...args: any[]) => mockUpdateBlock(...args),
    listPages: () => mockListPages(),
    // useTemplateCreation (architecture review #5) calls these on mount
    // / on submit. The BlockRow tests don't exercise the template
    // wizard, but the hook still mounts and fetches on first render,
    // so we have to stub the methods it touches.
    listTemplates: () => mockListTemplates(),
    createPageFromTemplate: (...args: any[]) => mockCreatePageFromTemplate(...args),
  },
}))

vi.mock('@shared/contexts/TabsContext', () => ({
  useTabs: () => ({ openTab: vi.fn() }),
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => vi.fn(),
}))

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
    wasmParseInline: (content: string) => mockParseInline(content),
    retry: vi.fn(),
  }),
  ensureWasmLoaded: vi.fn().mockResolvedValue(undefined),
}))

function makeBlock(content = '') {
  return {
    id: 'b1',
    pageId: 'p1',
    pageName: 'demo',
    content,
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId: null,
    order: 1,
    level: 0,
    collapsed: false,
    createdAt: '2026-06-02T00:00:00Z',
    updatedAt: '2026-06-02T00:00:00Z',
    properties: [],
  } as any
}

function renderRow(content = '') {
  const onUpdateSpy = vi.fn()

  function Wrapper() {
    const [block, setBlock] = useState(makeBlock(content))
    return (
      <BlockRow
        block={block}
        allBlocks={[block]}
        pageName="demo"
        hasChildren={false}
        isCollapsed={false}
        onToggleCollapse={vi.fn()}
        onUpdate={(updated) => {
          onUpdateSpy(updated)
          // Guard against an undefined payload from an unmocked
          // api.updateBlock — that would re-render BlockRow with
          // `block === undefined` and crash on the next state read.
          if (updated) setBlock(updated as any)
        }}
        onCreateBlock={vi.fn()}
        onDeleteBlock={vi.fn()}
        onFocusBlock={vi.fn()}
        onMoveBlockUp={vi.fn()}
        onMoveBlockDown={vi.fn()}
        onUndo={vi.fn()}
        onRedo={vi.fn()}
        indent={0}
      />
    )
  }

  render(<Wrapper />)
  return { onUpdate: onUpdateSpy }
}

describe('BlockRow editing behavior', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockParseInline.mockImplementation((content: string) => {
      if (content === '**hello**') {
        return {
          segments: [
            {
              Bold: {
                content: 'hello',
                raw: '**hello**',
                range: { start: 0, end: 9 },
              },
            },
          ],
        }
      }
      return { segments: [] }
    })
    Object.defineProperty(global.navigator, 'clipboard', {
      value: { writeText: mockWriteText },
      configurable: true,
    })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('saves exact text on blur without trimming spaces', async () => {
    const { onUpdate } = renderRow('old')
    mockUpdateBlock.mockResolvedValueOnce(makeBlock('  hello  '))

    const read = screen.getByText('old')
    fireEvent.click(read)

    const editor = screen.getByRole('textbox', { name: 'Block content' })
    editor.textContent = '  hello  '
    fireEvent.input(editor)
    fireEvent.blur(editor)

    await waitFor(() => {
      expect(mockUpdateBlock).toHaveBeenCalledWith('b1', { content: '  hello  ' })
    })
    expect(onUpdate).toHaveBeenCalled()
  })

  it('renders markdown once after blur (no duplicated raw text)', async () => {
    const { onUpdate } = renderRow('old')
    mockUpdateBlock.mockResolvedValueOnce(makeBlock('**hello**'))

    const read = screen.getByText('old')
    fireEvent.click(read)

    const editor = screen.getByRole('textbox', { name: 'Block content' })
    editor.textContent = '**hello**'
    fireEvent.input(editor)
    fireEvent.blur(editor)

    await waitFor(() => {
      expect(onUpdate).toHaveBeenCalled()
    })

    // The rendered view should show the formatted content only once.
    const rendered = await screen.findByText('hello')
    expect(rendered.tagName).toBe('STRONG')
    expect(screen.queryByText('**hello**')).not.toBeInTheDocument()
    expect(screen.getAllByText('hello')).toHaveLength(1)
  })

  // ──── # tag autocomplete (G4 from wikilinks audit) ─────────────

  it('opens the tag autocomplete when the user types "#t"', async () => {
    renderRow('')
    clickToEdit()

    const editor = screen.getByRole('textbox', { name: 'Block content' })
    editor.textContent = '#t'
    // Position the caret at the end of the content so handleInput
    // sees the trailing `#t` as the text before the cursor.
    setCaretPosition(editor, 2)
    fireEvent.input(editor)

    // The dropdown renders a listbox with tag options.
    const listbox = await screen.findByRole('listbox', { name: 'Tag suggestions' })
    expect(listbox).toBeInTheDocument()
    // The query "#t" matches the default "todo" tag.
    await screen.findByTestId('tag-option-todo')
  })

  it('filters the tag list by the prefix the user types', async () => {
    renderRow('')
    clickToEdit()

    const editor = screen.getByRole('textbox', { name: 'Block content' })
    editor.textContent = '#ur'
    setCaretPosition(editor, 3)
    fireEvent.input(editor)

    // Only "urgent" matches the prefix "ur".
    await screen.findByTestId('tag-option-urgent')
    expect(screen.queryByTestId('tag-option-todo')).not.toBeInTheDocument()
    expect(screen.queryByTestId('tag-option-bug')).not.toBeInTheDocument()
  })

  it('selecting a tag replaces the partial #partial with #tagname', async () => {
    renderRow('')
    clickToEdit()

    const editor = screen.getByRole('textbox', { name: 'Block content' })
    editor.textContent = '#urg'
    setCaretPosition(editor, 4)
    fireEvent.input(editor)

    // Wait for the dropdown to populate, then click the suggestion.
    const option = await screen.findByTestId('tag-option-urgent')
    fireEvent.click(option)

    // The editor should now contain the full `#urgent` tag.
    expect(editor.textContent).toBe('#urgent')
    // Dropdown should be gone after selection.
    expect(screen.queryByRole('listbox', { name: 'Tag suggestions' })).not.toBeInTheDocument()
  })

  it('does not open the tag autocomplete for markdown "## Heading"', async () => {
    renderRow('')
    clickToEdit()

    const editor = screen.getByRole('textbox', { name: 'Block content' })
    editor.textContent = '## Heading'
    setCaretPosition(editor, 10)
    fireEvent.input(editor)

    // The negative lookbehind keeps the second `#` from triggering.
    expect(screen.queryByRole('listbox', { name: 'Tag suggestions' })).not.toBeInTheDocument()
  })

  it('closes the tag dropdown on Escape', async () => {
    renderRow('')
    clickToEdit()

    const editor = screen.getByRole('textbox', { name: 'Block content' })
    editor.textContent = '#todo'
    setCaretPosition(editor, 5)
    fireEvent.input(editor)

    // Dropdown is open with the matching tag visible.
    await screen.findByTestId('tag-option-todo')

    // The editor handles Escape itself in handleKeyDown; in this
    // integration test we dispatch the keydown event on the editor.
    fireEvent.keyDown(editor, { key: 'Escape' })

    // Dropdown should be closed.
    expect(screen.queryByRole('listbox', { name: 'Tag suggestions' })).not.toBeInTheDocument()
  })
})

// ─── findNearestLink helper (Cmd/Ctrl+Enter target selection) ───────

describe('findNearestLink', () => {
  it('returns the page ref when cursor is inside [[Page]]', () => {
    expect(findNearestLink('hello [[MyPage]] world', 9)).toEqual({
      type: 'page',
      target: 'MyPage',
    })
  })

  it('strips the alias and returns the page name', () => {
    expect(findNearestLink('see [[MyPage|the pretty one]] please', 15)).toEqual({
      type: 'page',
      target: 'MyPage',
    })
  })

  it('returns the nearest link when cursor is between two links', () => {
    // [[A]]cursor[[B]]  — cursor at index 5 (after the closing ]] of A)
    const text = '[[A]]x[[B]]'
    expect(findNearestLink(text, 5)).toEqual({ type: 'page', target: 'A' })
    // At index 6, closer to B
    expect(findNearestLink(text, 6)).toEqual({ type: 'page', target: 'B' })
  })

  it('returns the block ref when cursor is inside ((block-id))', () => {
    expect(findNearestLink('ref ((abc-123)) end', 9)).toEqual({
      type: 'block',
      target: 'abc-123',
    })
  })

  it('returns the tag when cursor is on #tag', () => {
    expect(findNearestLink('idea #todo for tomorrow', 8)).toEqual({
      type: 'tag',
      target: 'todo',
    })
  })

  it('does not match # in the middle of a word', () => {
    // # inside an email-like token should not be picked up
    expect(findNearestLink('user#foo', 5)).toBeNull()
  })

  it('returns null when there is no link near the cursor', () => {
    expect(findNearestLink('just plain text with no links', 5)).toBeNull()
  })
})
