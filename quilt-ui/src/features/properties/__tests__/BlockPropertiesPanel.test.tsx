import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { BlockPropertiesPanel } from '../BlockPropertiesPanel'
import type { BlockProperty } from '@shared/types/api'

// Mock the api-client so panel save/load behaviour is observable.
const mockGetBlockProperties = vi.fn()
const mockSetBlockProperty = vi.fn()
const mockDeleteBlockProperty = vi.fn()
const mockListPropertyKeys = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    getBlockProperties: (...args: any[]) => mockGetBlockProperties(...args),
    setBlockProperty: (...args: any[]) => mockSetBlockProperty(...args),
    deleteBlockProperty: (...args: any[]) => mockDeleteBlockProperty(...args),
    listPropertyKeys: (...args: any[]) => mockListPropertyKeys(...args),
  },
}))

function makeProp(key: string, value: string | number | boolean | null, type: BlockProperty['type'] = 'string'): BlockProperty {
  return { key, value, type }
}

function renderPanel(blockId = 'b1', onClose = vi.fn()) {
  render(<BlockPropertiesPanel blockId={blockId} onClose={onClose} />)
  return { onClose }
}

describe('BlockPropertiesPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockGetBlockProperties.mockResolvedValue([])
    mockSetBlockProperty.mockResolvedValue(undefined)
    mockDeleteBlockProperty.mockResolvedValue(undefined)
    mockListPropertyKeys.mockResolvedValue({ keys: [], nextCursor: null })
  })

  afterEach(() => {
    cleanup()
  })

  it('loads properties on mount and renders the empty state when none', async () => {
    renderPanel()
    await waitFor(() => {
      expect(mockGetBlockProperties).toHaveBeenCalledWith('b1')
    })
    expect(await screen.findByText(/no properties yet/i)).toBeInTheDocument()
  })

  it('renders all properties returned by the API', async () => {
    mockGetBlockProperties.mockResolvedValueOnce([
      makeProp('status', 'open', 'select'),
      makeProp('priority', 'A', 'select'),
    ])
    renderPanel()
    expect(await screen.findByText('status')).toBeInTheDocument()
    expect(await screen.findByText('priority')).toBeInTheDocument()
  })

  it('calls api.setBlockProperty when a property value changes', async () => {
    mockGetBlockProperties.mockResolvedValueOnce([makeProp('status', 'open', 'select')])
    renderPanel()

    const input = await screen.findByDisplayValue('open')
    fireEvent.change(input, { target: { value: 'closed' } })

    await waitFor(() => {
      expect(mockSetBlockProperty).toHaveBeenCalledWith('b1', 'status', 'closed')
    })
  })

  it('adds a new property and persists it via setBlockProperty', async () => {
    mockGetBlockProperties.mockResolvedValueOnce([])
    renderPanel()

    // Open the add form
    const addBtn = await screen.findByRole('button', { name: /add property/i })
    fireEvent.click(addBtn)

    const keyInput = await screen.findByPlaceholderText(/property name/i)
    fireEvent.change(keyInput, { target: { value: 'deadline' } })
    fireEvent.keyDown(keyInput, { key: 'Enter' })

    await waitFor(() => {
      expect(mockSetBlockProperty).toHaveBeenCalledWith('b1', 'deadline', '')
    })
  })

  it('removes a property by calling deleteBlockProperty when the X is clicked', async () => {
    mockGetBlockProperties.mockResolvedValueOnce([makeProp('status', 'open', 'select')])
    renderPanel()

    const removeBtn = await screen.findByRole('button', { name: /delete property status/i })
    fireEvent.click(removeBtn)

    await waitFor(() => {
      expect(mockDeleteBlockProperty).toHaveBeenCalledWith('b1', 'status')
    })
  })

  it('resolves natural-date tokens (today / tomorrow / yesterday) before saving', async () => {
    // Pin "now" so the resolver is deterministic. We mock
    // `Date.now` instead of using vi.useFakeTimers so we don't
    // interfere with the React scheduler / react-hot-toast timers
    // that other tests rely on.
    const realDateNow = Date.now
    const realDateCtor = Date
    const refMs = realDateCtor.parse('2026-06-05T12:00:00Z')
    class FixedDate extends realDateCtor {
      constructor(...args: ConstructorParameters<typeof Date>) {
        if (args.length === 0) {
          super(refMs)
        } else {
          // @ts-expect-error — spread through to super
          super(...args)
        }
      }
      static now() {
        return refMs
      }
    }
    // @ts-expect-error — global Date swap for this test only
    globalThis.Date = FixedDate
    try {
      mockGetBlockProperties.mockResolvedValueOnce([makeProp('deadline', '2026-06-01', 'date')])
      renderPanel()
      const input = await screen.findByDisplayValue('2026-06-01')
      fireEvent.change(input, { target: { value: 'tomorrow' } })

      await waitFor(() => {
        expect(mockSetBlockProperty).toHaveBeenCalledWith('b1', 'deadline', '2026-06-06')
      })
    } finally {
      // @ts-expect-error — restore
      globalThis.Date = realDateCtor
    }
  })

  it('leaves the value unchanged if the user did NOT type a natural date', async () => {
    mockGetBlockProperties.mockResolvedValueOnce([makeProp('status', 'open', 'select')])
    renderPanel()

    const input = await screen.findByDisplayValue('open')
    fireEvent.change(input, { target: { value: 'in-progress' } })

    await waitFor(() => {
      expect(mockSetBlockProperty).toHaveBeenCalledWith('b1', 'status', 'in-progress')
    })
  })

  it('renders boolean properties as checkboxes', async () => {
    mockGetBlockProperties.mockResolvedValueOnce([makeProp('done', true, 'boolean')])
    renderPanel()
    // The existing panel renders the boolean as a plain <input
    // type="checkbox"> with no aria-label. There is exactly one
    // checkbox on the page, so a container query is the simplest
    // non-fragile selector.
    const checkbox = (await waitFor(() => {
      const el = document.querySelector('input[type="checkbox"]') as HTMLInputElement | null
      if (!el) throw new Error('checkbox not found')
      return el
    }))
    expect(checkbox.checked).toBe(true)
  })
})
