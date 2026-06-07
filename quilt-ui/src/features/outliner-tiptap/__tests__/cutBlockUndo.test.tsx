/**
 * Integration test: Cmd+X cuts a block, Cmd+Z brings it back.
 *
 * This is the user-facing contract for the cut-block + undo feature.
 * The full chain is:
 *
 *   1. User focuses a block in BlockRow
 *   2. User hits Cmd+X → blockKeyboardHandler returns `CutBlock`
 *   3. BlockRow's adapter calls `onCutBlock(snapshot)`
 *   4. The parent (a small harness in this test, but PageView in
 *      production) pushes a `restore` action onto the UndoManager,
 *      deletes the block via the API, and removes it from local state
 *   5. User hits Cmd+Z → page-level handler tries UndoManager.undo()
 *      first, which calls `restore` → recreate block via the API,
 *      re-insert in local state
 *
 * The test asserts behavior at each step (block present → cut → block
 * gone → undo → block back), not implementation details like the
 * number of internal calls. We mock the API at the boundary so the
 * test runs offline, but the *contract* is end-to-end.
 */
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react'
import { useState } from 'react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { BlockRow } from '../BlockRow'
import { UndoManager } from '@shared/hooks/useUndoManager'
import type { Block } from '@shared/types/api'

// ──── API mocks ────────────────────────────────────────────────────────
// The harness talks to the real `api` object; the api module is mocked
// at the module level. We capture calls so the test can assert
// behavior (delete called on cut, create called on undo) without
// reaching for a real network.

const mockCreateBlock = vi.fn()
const mockDeleteBlock = vi.fn()
const mockUpdateBlock = vi.fn()
const mockListPages = vi.fn().mockResolvedValue([])
const mockListTemplates = vi.fn().mockResolvedValue([])
const mockCreatePageFromTemplate = vi.fn()
const mockWriteText = vi.fn().mockResolvedValue(undefined)
const mockParseInline = vi.fn((content: string) => ({ segments: [] }))

vi.mock('@core/api-client', () => ({
  api: {
    createBlock: (...args: any[]) => mockCreateBlock(...args),
    deleteBlock: (...args: any[]) => mockDeleteBlock(...args),
    updateBlock: (...args: any[]) => mockUpdateBlock(...args),
    listPages: () => mockListPages(),
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

// ──── Harness ──────────────────────────────────────────────────────────
//
// Renders a BlockRow with a real UndoManager and the cut/undo wiring
// that PageView uses. The harness is intentionally minimal — it's the
// thinnest possible shell that exercises the full contract.

function makeBlock(overrides: Partial<Block> = {}): Block {
  return {
    id: 'b1',
    pageId: 'p1',
    pageName: 'demo',
    content: 'Hello world',
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
    ...overrides,
  } as Block
}

interface HarnessProps {
  initialBlocks: Block[]
  /** Simulated server response for createBlock. */
  createdBlock: Block
  /** PageView passes its pageName into the create call. */
  pageName?: string
}

function CutUndoHarness({ initialBlocks, createdBlock, pageName = 'demo' }: HarnessProps) {
  const [blocks, setBlocks] = useState<Block[]>(initialBlocks)
  const undoManager = useState(() => new UndoManager(50))[0]

  const handleCutBlock = async (snapshot: Block) => {
    // 1) push restore
    undoManager.push({
      type: 'cut-block',
      restore: async () => {
        const created = await mockCreateBlock({
          pageName: snapshot.pageName ?? pageName,
          content: snapshot.content,
          parentId: snapshot.parentId,
          precedingBlockId: undefined,
        })
        // server may return a new id; use the harness's predetermined
        // result so the test can assert on it.
        const restored = (created as Block) ?? createdBlock
        setBlocks(prev =>
          prev.some(b => b.id === restored.id) ? prev : [...prev, restored],
        )
      },
    })
    // 2) clipboard + delete + state
    mockWriteText(snapshot.content)
    setBlocks(prev => prev.filter(b => b.id !== snapshot.id))
    await mockDeleteBlock(snapshot.id)
  }

  const handleUndo = async () => {
    if (undoManager.canUndo()) {
      const ok = await undoManager.undo()
      if (ok) return
    }
    // No WASM fallback needed for this test.
  }

  if (blocks.length === 0) {
    return (
      <div>
        <div data-testid="empty">no blocks</div>
        <button data-testid="undo-btn" onClick={handleUndo}>undo</button>
      </div>
    )
  }

  return (
    <div>
      <div data-testid="block-list">
        {blocks.map(b => (
          <BlockRow
            key={b.id}
            block={b}
            allBlocks={blocks}
            pageName={pageName}
            hasChildren={false}
            isCollapsed={false}
            onToggleCollapse={vi.fn()}
            onUpdate={vi.fn()}
            onCreateBlock={vi.fn()}
            onDeleteBlock={vi.fn()}
            onFocusBlock={vi.fn()}
            onMoveBlockUp={vi.fn()}
            onMoveBlockDown={vi.fn()}
            onUndo={handleUndo}
            onRedo={vi.fn()}
            indent={0}
            onCutBlock={handleCutBlock}
          />
        ))}
      </div>
      <button data-testid="undo-btn" onClick={handleUndo}>undo</button>
    </div>
  )
}

// ──── Helpers ──────────────────────────────────────────────────────────

function clickToEdit() {
  const read = document.querySelector('.block-content-read') as HTMLElement | null
  expect(read).not.toBeNull()
  fireEvent.click(read!)
}

function getEditor(): HTMLElement {
  return screen.getByRole('textbox', { name: 'Block content' })
}

// ──── Tests ───────────────────────────────────────────────────────────

describe('CutBlock + UndoManager', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    Object.defineProperty(global.navigator, 'clipboard', {
      value: { writeText: mockWriteText },
      configurable: true,
    })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('Cmd+X on a block calls onCutBlock with a snapshot and removes the block', async () => {
    const block = makeBlock({ id: 'b-cut-me', content: 'to be cut' })
    mockDeleteBlock.mockResolvedValueOnce({ deleted: true })

    render(<CutUndoHarness initialBlocks={[block]} createdBlock={block} />)

    // The block is in the DOM and editable.
    expect(screen.getByText('to be cut')).toBeInTheDocument()
    clickToEdit()
    const editor = getEditor()

    // Fire Cmd+X — no text selection, so the pure handler returns CutBlock.
    await act(async () => {
      fireEvent.keyDown(editor, { key: 'x', metaKey: true })
    })

    await waitFor(() => {
      // The block must be removed from the DOM.
      expect(screen.queryByText('to be cut')).not.toBeInTheDocument()
    })

    // The harness writes to clipboard + deletes via the API.
    expect(mockWriteText).toHaveBeenCalledWith('to be cut')
    expect(mockDeleteBlock).toHaveBeenCalledWith('b-cut-me')
  })

  it('Cmd+Z (undo) brings the cut block back via the API', async () => {
    const original = makeBlock({ id: 'b-restore', content: 'come back soon' })
    // The server replies with a new block (it has a new id).
    const recreated = makeBlock({ id: 'b-restore', content: 'come back soon' })
    mockDeleteBlock.mockResolvedValueOnce({ deleted: true })
    mockCreateBlock.mockResolvedValueOnce(recreated)

    render(<CutUndoHarness initialBlocks={[original]} createdBlock={recreated} />)

    clickToEdit()
    const editor = getEditor()

    // Cut it.
    await act(async () => {
      fireEvent.keyDown(editor, { key: 'x', metaKey: true })
    })
    await waitFor(() => {
      expect(screen.queryByText('come back soon')).not.toBeInTheDocument()
    })

    // Undo via the page-level button (same code path as Cmd+Z in
    // PageView's global keydown handler).
    await act(async () => {
      fireEvent.click(screen.getByTestId('undo-btn'))
    })

    // The block reappears and the API was called to recreate it.
    await waitFor(() => {
      expect(screen.getByText('come back soon')).toBeInTheDocument()
    })
    expect(mockCreateBlock).toHaveBeenCalledTimes(1)
    expect(mockCreateBlock).toHaveBeenCalledWith(
      expect.objectContaining({ content: 'come back soon' }),
    )
  })

  it('after cutting, the UndoManager has one action; after undo, zero', async () => {
    const block = makeBlock({ id: 'b-count', content: 'count me' })
    mockDeleteBlock.mockResolvedValueOnce({ deleted: true })
    mockCreateBlock.mockResolvedValueOnce(block)

    render(<CutUndoHarness initialBlocks={[block]} createdBlock={block} />)

    clickToEdit()
    const editor = getEditor()

    // We can observe the manager size via the absence of `canUndo`
    // on the empty-stack undo button: clicking it with no stack must
    // be a no-op (no API call to createBlock). Before the cut, the
    // undo button should be a no-op.
    const undoBtn = screen.getByTestId('undo-btn')

    await act(async () => {
      fireEvent.click(undoBtn)
    })
    expect(mockCreateBlock).not.toHaveBeenCalled()

    // Cut.
    await act(async () => {
      fireEvent.keyDown(editor, { key: 'x', metaKey: true })
    })
    await waitFor(() => {
      expect(screen.queryByText('count me')).not.toBeInTheDocument()
    })

    // Undo — should call createBlock once.
    await act(async () => {
      fireEvent.click(undoBtn)
    })
    await waitFor(() => {
      expect(mockCreateBlock).toHaveBeenCalledTimes(1)
    })

    // Undo again — should be a no-op because the stack is empty.
    await act(async () => {
      fireEvent.click(undoBtn)
    })
    // The second undo must not call the API again.
    expect(mockCreateBlock).toHaveBeenCalledTimes(1)
  })

  it('Cmd+X with no active text selection cuts the block (browser-native cut is bypassed)', async () => {
    // Sanity guard: the simple "no selection" case is the path that
    // our handler actually owns. The "active text selection" path is
    // already covered by the pure `blockKeyboardHandler` unit tests
    // (where jsdom selection handling is irrelevant). Here we just
    // confirm the integration wiring on the no-selection path.
    const block = makeBlock({ id: 'b-no-sel', content: 'plain text' })
    mockDeleteBlock.mockResolvedValueOnce({ deleted: true })

    render(<CutUndoHarness initialBlocks={[block]} createdBlock={block} />)

    clickToEdit()
    const editor = getEditor()
    // Make sure no selection is active.
    window.getSelection()?.removeAllRanges()

    await act(async () => {
      fireEvent.keyDown(editor, { key: 'x', metaKey: true })
    })

    await waitFor(() => {
      expect(screen.queryByText('plain text')).not.toBeInTheDocument()
    })
    expect(mockDeleteBlock).toHaveBeenCalledWith('b-no-sel')
  })

  it('snapshot includes unsaved local edits at the moment of cut', async () => {
    // The user types in the editor but doesn't blur; the cut snapshot
    // should carry the *typed* content, not the stale `block.content`.
    const block = makeBlock({ id: 'b-unsaved', content: 'original' })
    mockDeleteBlock.mockResolvedValueOnce({ deleted: true })

    render(<CutUndoHarness initialBlocks={[block]} createdBlock={block} />)

    clickToEdit()
    const editor = getEditor()

    // Type new content (no blur) — this lives in localContent.
    editor.textContent = 'updated text'
    fireEvent.input(editor)

    // Cut.
    await act(async () => {
      fireEvent.keyDown(editor, { key: 'x', metaKey: true })
    })

    await waitFor(() => {
      // The clipboard write and the API delete both reflect the
      // updated content, not the original.
      expect(mockWriteText).toHaveBeenCalledWith('updated text')
      expect(mockDeleteBlock).toHaveBeenCalledWith('b-unsaved')
    })
  })
})
