import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { BlockContextMenu, type BlockContextMenuActions } from '../BlockContextMenu'

// The menu uses getBoundingClientRect to position itself; mock it so the
// calculation has a stable geometry to anchor against.
function makeAnchor(): HTMLElement {
  const anchor = document.createElement('button')
  anchor.textContent = 'trigger'
  document.body.appendChild(anchor)
  // jsdom returns zeros for getBoundingClientRect; provide a known rect.
  anchor.getBoundingClientRect = () => ({
    top: 100,
    bottom: 130,
    left: 50,
    right: 80,
    width: 30,
    height: 30,
    x: 50,
    y: 100,
    toJSON: () => ({}),
  })
  return anchor
}

const ACTIONS: BlockContextMenuActions = {
  onAddChild: vi.fn(),
  onMoveUp: vi.fn(),
  onMoveDown: vi.fn(),
  onConvertToTask: vi.fn(),
  onCopyLink: vi.fn(),
  onDelete: vi.fn(),
}

describe('BlockContextMenu — DESIGN.md §11.3', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('does not render when closed', () => {
    const anchor = makeAnchor()
    render(
      <BlockContextMenu open={false} anchorEl={anchor} onClose={vi.fn()} actions={ACTIONS} />,
    )
    expect(screen.queryByRole('menu')).not.toBeInTheDocument()
    anchor.remove()
  })

  it('does not render without an anchor', () => {
    render(
      <BlockContextMenu open={true} anchorEl={null} onClose={vi.fn()} actions={ACTIONS} />,
    )
    expect(screen.queryByRole('menu')).not.toBeInTheDocument()
  })

  it('renders all 6 required actions when open', () => {
    const anchor = makeAnchor()
    render(
      <BlockContextMenu open={true} anchorEl={anchor} onClose={vi.fn()} actions={ACTIONS} />,
    )
    expect(screen.getByRole('menu')).toBeInTheDocument()
    expect(screen.getByRole('menuitem', { name: /Add child block/ })).toBeInTheDocument()
    expect(screen.getByRole('menuitem', { name: /Move up/ })).toBeInTheDocument()
    expect(screen.getByRole('menuitem', { name: /Move down/ })).toBeInTheDocument()
    expect(screen.getByRole('menuitem', { name: /Convert to task/ })).toBeInTheDocument()
    expect(screen.getByRole('menuitem', { name: /Copy block link/ })).toBeInTheDocument()
    expect(screen.getByRole('menuitem', { name: /Delete block/ })).toBeInTheDocument()
    anchor.remove()
  })

  it('invokes onAddChild and closes when "Add child block" is clicked', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    const anchor = makeAnchor()
    render(
      <BlockContextMenu open={true} anchorEl={anchor} onClose={onClose} actions={ACTIONS} />,
    )
    await user.click(screen.getByRole('menuitem', { name: /Add child block/ }))
    expect(ACTIONS.onAddChild).toHaveBeenCalledTimes(1)
    expect(onClose).toHaveBeenCalledTimes(1)
    anchor.remove()
  })

  it('invokes onMoveUp, onMoveDown, onConvertToTask, onCopyLink, onDelete correctly', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    const anchor = makeAnchor()
    render(
      <BlockContextMenu open={true} anchorEl={anchor} onClose={onClose} actions={ACTIONS} />,
    )

    await user.click(screen.getByRole('menuitem', { name: /Move up/ }))
    expect(ACTIONS.onMoveUp).toHaveBeenCalledTimes(1)

    await user.click(screen.getByRole('menuitem', { name: /Move down/ }))
    expect(ACTIONS.onMoveDown).toHaveBeenCalledTimes(1)

    await user.click(screen.getByRole('menuitem', { name: /Convert to task/ }))
    expect(ACTIONS.onConvertToTask).toHaveBeenCalledTimes(1)

    await user.click(screen.getByRole('menuitem', { name: /Copy block link/ }))
    expect(ACTIONS.onCopyLink).toHaveBeenCalledTimes(1)

    await user.click(screen.getByRole('menuitem', { name: /Delete block/ }))
    expect(ACTIONS.onDelete).toHaveBeenCalledTimes(1)

    // Each click also closes the menu.
    expect(onClose).toHaveBeenCalledTimes(5)
    anchor.remove()
  })

  it('closes when Escape is pressed', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    const anchor = makeAnchor()
    render(
      <BlockContextMenu open={true} anchorEl={anchor} onClose={onClose} actions={ACTIONS} />,
    )
    await user.keyboard('{Escape}')
    expect(onClose).toHaveBeenCalledTimes(1)
    anchor.remove()
  })

  it('closes when clicking outside the menu and the anchor', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    const anchor = makeAnchor()
    render(
      <BlockContextMenu open={true} anchorEl={anchor} onClose={onClose} actions={ACTIONS} />,
    )
    // Click somewhere outside (e.g. body)
    await user.click(document.body)
    expect(onClose).toHaveBeenCalledTimes(1)
    anchor.remove()
  })

  it('Delete item is styled as destructive (red text)', () => {
    const anchor = makeAnchor()
    render(
      <BlockContextMenu open={true} anchorEl={anchor} onClose={vi.fn()} actions={ACTIONS} />,
    )
    const deleteBtn = screen.getByRole('menuitem', { name: /Delete block/ })
    expect(deleteBtn.style.color).toBe('var(--color-danger)')
    anchor.remove()
  })

  // ── F3 of quilt-fase2-ux-dead-buttons ───────────────────────────
  // The Properties action is the discoverable entry point for the
  // BlockPropertiesPanel (which was previously only reachable via
  // a hover-revealed Settings2 button on the row).

  it('does not render a Properties item when onShowProperties is omitted', () => {
    const anchor = makeAnchor()
    render(
      <BlockContextMenu open={true} anchorEl={anchor} onClose={vi.fn()} actions={ACTIONS} />,
    )
    // Sanity: only the original 6 items are present.
    expect(screen.queryByRole('menuitem', { name: /^Properties$/ })).not.toBeInTheDocument()
    expect(screen.getAllByRole('menuitem')).toHaveLength(6)
    anchor.remove()
  })

  it('renders a Properties item that calls onShowProperties and closes the menu', async () => {
    const user = userEvent.setup()
    const onShowProperties = vi.fn()
    const onClose = vi.fn()
    const anchor = makeAnchor()
    const actionsWithProps: BlockContextMenuActions = {
      ...ACTIONS,
      onShowProperties,
    }
    render(
      <BlockContextMenu
        open={true}
        anchorEl={anchor}
        onClose={onClose}
        actions={actionsWithProps}
      />,
    )
    const propsBtn = screen.getByRole('menuitem', { name: /^Properties$/ })
    expect(propsBtn).toBeInTheDocument()
    // Total item count grows by exactly one.
    expect(screen.getAllByRole('menuitem')).toHaveLength(7)

    await user.click(propsBtn)
    expect(onShowProperties).toHaveBeenCalledTimes(1)
    // Same auto-close contract as the other actions.
    expect(onClose).toHaveBeenCalledTimes(1)
    anchor.remove()
  })
})
