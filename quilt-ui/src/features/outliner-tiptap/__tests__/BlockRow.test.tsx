import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { useState } from 'react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { BlockRow } from '../BlockRow'

const mockUpdateBlock = vi.fn()
const mockListPages = vi.fn().mockResolvedValue([])
const mockWriteText = vi.fn().mockResolvedValue(undefined)
const mockParseInline = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    updateBlock: (...args: any[]) => mockUpdateBlock(...args),
    listPages: () => mockListPages(),
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
          setBlock(updated as any)
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
})
