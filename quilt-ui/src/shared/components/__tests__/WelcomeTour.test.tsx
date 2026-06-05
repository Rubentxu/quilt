// ─── WelcomeTour — first-run product tour (F3) ────────────────────
//
// Approval tests for the first-run modal that explains the four
// key Quilt primitives.
//
// Spec: F3 of `quilt-fase2-ux-empty-states`.
// Contract:
//   - Renders 4 feature cards (Plantillas, Recientes, Slash
//     command, Properties) with an icon, title and short body.
//   - The close button (X icon) and the "Got it" CTA both:
//     a) Set `STORAGE_KEYS.WELCOME_SEEN` (`'quilt-welcome-seen'`)
//        to `'1'` in localStorage.
//     b) Invoke the `onClose` callback so the AppShell can hide
//        the dialog.
//   - Escape keypress also dismisses the tour with the same
//     effects.
//   - Backdrop click dismisses the tour; click on the dialog
//     body does NOT (so users can interact with the cards without
//     accidentally closing).

import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { WelcomeTour } from '../WelcomeTour'
import { STORAGE_KEYS } from '@features/sidebar/storage-keys'

beforeEach(() => {
  localStorage.clear()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('WelcomeTour — F3 of quilt-fase2-ux-empty-states', () => {
  it('renders a dialog with role=dialog and aria-labelledby pointing at the title', () => {
    const onClose = vi.fn()
    render(<WelcomeTour onClose={onClose} />)

    const dialog = screen.getByRole('dialog')
    expect(dialog).toBeInTheDocument()
    // The label is provided via aria-labelledby → the title node.
    expect(dialog).toHaveAttribute('aria-labelledby', 'welcome-tour-title')
    expect(dialog).toHaveAttribute('aria-modal', 'true')
  })

  it('renders the "Welcome to Quilt" title', () => {
    const onClose = vi.fn()
    render(<WelcomeTour onClose={onClose} />)
    expect(
      screen.getByRole('heading', { name: /welcome to quilt/i }),
    ).toBeInTheDocument()
  })

  it('renders all six feature cards with their titles', () => {
    const onClose = vi.fn()
    render(<WelcomeTour onClose={onClose} />)

    // Each card has a data-testid of the form
    // `welcome-tour-card-<title-lowercased>` and contains a heading
    // with the card title.
    // Spec: F1 of `quilt-fase4-onboarding-advanced` extends the
    // four-card tour with two advanced cards: Cognitive and MCP
    // tools.
    expect(screen.getByTestId('welcome-tour-card-plantillas')).toBeInTheDocument()
    expect(screen.getByTestId('welcome-tour-card-recientes')).toBeInTheDocument()
    expect(screen.getByTestId('welcome-tour-card-slash command')).toBeInTheDocument()
    expect(screen.getByTestId('welcome-tour-card-properties')).toBeInTheDocument()
    expect(screen.getByTestId('welcome-tour-card-cognitive')).toBeInTheDocument()
    expect(screen.getByTestId('welcome-tour-card-mcp tools')).toBeInTheDocument()

    // The card titles are inside the testid nodes as plain text —
    // checking by testid is enough to prove the six cards are
    // present.
    expect(screen.getByTestId('welcome-tour-card-plantillas')).toHaveTextContent('Plantillas')
    expect(screen.getByTestId('welcome-tour-card-recientes')).toHaveTextContent('Recientes')
    expect(screen.getByTestId('welcome-tour-card-cognitive')).toHaveTextContent('Cognitive')
    expect(screen.getByTestId('welcome-tour-card-mcp tools')).toHaveTextContent('MCP tools')
  })

  it('renders the Cognitive card explaining the agent activity panel', () => {
    // Spec: F1 of `quilt-fase4-onboarding-advanced` — the Cognitive
    // card tells new users about the Agent Activity panel in the
    // sidebar. Body text must mention the panel and how to open it.
    const onClose = vi.fn()
    render(<WelcomeTour onClose={onClose} />)

    const card = screen.getByTestId('welcome-tour-card-cognitive')
    expect(card).toBeInTheDocument()
    // Title present (proves the card was rendered, not just an
    // empty <div>).
    expect(card).toHaveTextContent('Cognitive')
    // Body text contains the sidebar hint (case-insensitive — the
    // exact casing of the entry-point label may evolve).
    expect(card.textContent?.toLowerCase()).toContain('agent activity')
    expect(card.textContent?.toLowerCase()).toContain('sidebar')
  })

  it('renders the MCP tools card explaining Model Context Protocol exposure', () => {
    // Spec: F1 of `quilt-fase4-onboarding-advanced` — the MCP
    // tools card tells new users that Quilt exposes itself to AI
    // agents via the Model Context Protocol. Body must mention
    // both the protocol and where to read the full tool list.
    const onClose = vi.fn()
    render(<WelcomeTour onClose={onClose} />)

    const card = screen.getByTestId('welcome-tour-card-mcp tools')
    expect(card).toBeInTheDocument()
    // Title present.
    expect(card).toHaveTextContent('MCP tools')
    // Body mentions Model Context Protocol (the canonical name
    // of the standard the card is describing) and that agents can
    // call tools.
    expect(card.textContent?.toLowerCase()).toContain('model context protocol')
    expect(card.textContent?.toLowerCase()).toContain('agent')
  })

  it('the "Got it" button sets the WELCOME_SEEN flag and calls onClose', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<WelcomeTour onClose={onClose} />)

    await user.click(screen.getByTestId('welcome-tour-got-it'))

    expect(localStorage.getItem(STORAGE_KEYS.WELCOME_SEEN)).toBe('1')
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('the X close button sets the WELCOME_SEEN flag and calls onClose', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<WelcomeTour onClose={onClose} />)

    await user.click(screen.getByTestId('welcome-tour-close'))

    expect(localStorage.getItem(STORAGE_KEYS.WELCOME_SEEN)).toBe('1')
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('Escape keypress dismisses the tour (sets flag + calls onClose)', async () => {
    const onClose = vi.fn()
    render(<WelcomeTour onClose={onClose} />)

    // The component installs a keydown listener on mount; fire one.
    fireEvent.keyDown(document, { key: 'Escape' })

    expect(localStorage.getItem(STORAGE_KEYS.WELCOME_SEEN)).toBe('1')
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('clicking the backdrop dismisses the tour; clicking the dialog body does not', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<WelcomeTour onClose={onClose} />)

    // Click on a card body — should NOT close.
    await user.click(screen.getByTestId('welcome-tour-card-plantillas'))
    expect(onClose).not.toHaveBeenCalled()
    expect(localStorage.getItem(STORAGE_KEYS.WELCOME_SEEN)).toBeNull()

    // Click on the backdrop wrapper. The component uses a
    // presentation-role wrapper as the backdrop; the dialog is
    // nested inside.
    const dialog = screen.getByRole('dialog')
    const backdrop = dialog.parentElement
    expect(backdrop).toBeTruthy()
    fireEvent.click(backdrop!, { target: backdrop } as unknown as MouseEvent)

    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1)
    })
    expect(localStorage.getItem(STORAGE_KEYS.WELCOME_SEEN)).toBe('1')
  })

  it('still calls onClose when localStorage.setItem throws (defensive)', async () => {
    // Simulate quota / private-mode failure. The component must
    // still notify the parent so the dialog unmounts, otherwise
    // the user is stuck staring at the modal.
    const setItemSpy = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new Error('quota exceeded')
      })
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<WelcomeTour onClose={onClose} />)

    await user.click(screen.getByTestId('welcome-tour-got-it'))

    expect(onClose).toHaveBeenCalledTimes(1)
    setItemSpy.mockRestore()
  })
})

// ─── F2 of quilt-fase3-backlog-small-fixes — focus trap ───────────
//
// The dialog has two focusable elements (close button, "Got it"
// CTA). The 4 feature cards are divs, not focusable. The focus
// trap keeps keyboard users inside the dialog by wrapping Tab
// from the last element back to the first, and Shift+Tab from
// the first back to the last.
//
// We test the trap by dispatching keyboard events on the focus
// target we expect to wrap from. The component's handler then
// moves `document.activeElement` to the wrapped target.

describe('WelcomeTour — F2 (focus trap)', () => {
  it('Tab on the last focusable element (Got it) wraps to the first (close button)', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<WelcomeTour onClose={onClose} />)

    // Move focus to "Got it" — the last focusable element. We
    // .focus() it directly; the component auto-focuses the close
    // button on mount, and a programmatic focus is what a Tab
    // keypress would have landed us on in a real browser.
    const gotIt = screen.getByTestId('welcome-tour-got-it')
    gotIt.focus()
    expect(document.activeElement).toBe(gotIt)

    // Press Tab — without the trap, focus would move to
    // document.body or to a sibling of the dialog. With the
    // trap, it must wrap to the close button.
    await user.keyboard('{Tab}')

    const closeBtn = screen.getByTestId('welcome-tour-close')
    expect(document.activeElement).toBe(closeBtn)
    // The trap did NOT dismiss the dialog.
    expect(onClose).not.toHaveBeenCalled()
  })

  it('Shift+Tab on the first focusable element (close button) wraps to the last (Got it)', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<WelcomeTour onClose={onClose} />)

    // The component auto-focuses the close button on mount.
    const closeBtn = screen.getByTestId('welcome-tour-close')
    expect(document.activeElement).toBe(closeBtn)

    await user.keyboard('{Shift>}{Tab}{/Shift}')

    const gotIt = screen.getByTestId('welcome-tour-got-it')
    expect(document.activeElement).toBe(gotIt)
    expect(onClose).not.toHaveBeenCalled()
  })

  it('Tab from outside the dialog pulls focus back inside the dialog', async () => {
    // Simulate the edge case: focus has somehow escaped the
    // dialog (e.g. via a programmatic .focus() to a sibling
    // element). The next Tab should pull focus back into the
    // dialog rather than letting the user tab into the
    // underlying app.
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<WelcomeTour onClose={onClose} />)

    // Create a focusable sibling OUTSIDE the dialog and move
    // focus there. This simulates a real-world bug where the
    // initial render race let focus escape.
    const outsideButton = document.createElement('button')
    outsideButton.textContent = 'Outside the dialog'
    outsideButton.setAttribute('data-testid', 'outside-button')
    document.body.appendChild(outsideButton)
    outsideButton.focus()
    expect(document.activeElement).toBe(outsideButton)

    // Verify it really is outside the dialog.
    const dialog = screen.getByRole('dialog')
    expect(dialog.contains(outsideButton)).toBe(false)

    await user.keyboard('{Tab}')

    // Focus must now be on a focusable element INSIDE the dialog.
    expect(dialog.contains(document.activeElement)).toBe(true)
    expect(onClose).not.toHaveBeenCalled()

    // Cleanup
    document.body.removeChild(outsideButton)
  })

  it('restores focus to the previously-focused element on unmount', async () => {
    // Set up an element outside the dialog to be the focus
    // "owner" — the kebab menu in the top bar, in real usage.
    const trigger = document.createElement('button')
    trigger.textContent = 'Kebab menu'
    trigger.setAttribute('data-testid', 'external-trigger')
    document.body.appendChild(trigger)
    trigger.focus()
    expect(document.activeElement).toBe(trigger)

    const onClose = vi.fn()
    const { unmount } = render(<WelcomeTour onClose={onClose} />)

    // The component auto-focuses the close button on mount.
    const closeBtn = screen.getByTestId('welcome-tour-close')
    expect(document.activeElement).toBe(closeBtn)

    // Dismount the dialog (e.g. user clicks Got it).
    unmount()

    // Focus must be restored to the original trigger.
    expect(document.activeElement).toBe(trigger)

    // Cleanup
    document.body.removeChild(trigger)
  })
})

// ─── B of quilt-fase4-cross-device-tour — server-synced dismissal ────
//
// The localStorage flag is the fast cache; the server is the source
// of truth so a dismissal on desktop also hides the tour on mobile.
// These tests pin the wire-level contract: on every dismiss path
// (Got it, X button, Escape, backdrop) the component MUST call
// `api.dismissTour('welcome')` AND continue to set the localStorage
// flag. The server call is fire-and-forget — a network blip must
// not block the UI or the in-memory state.

import { api } from '@core/api-client'

describe('WelcomeTour — B of quilt-fase4-cross-device-tour (server sync)', () => {
  let dismissSpy: ReturnType<typeof vi.fn>

  beforeEach(() => {
    dismissSpy = vi
      .spyOn(api, 'dismissTour')
      .mockResolvedValue({ dismissed: ['welcome'] })
  })

  it('"Got it" calls api.dismissTour("welcome") and sets the localStorage flag', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<WelcomeTour onClose={onClose} />)

    await user.click(screen.getByTestId('welcome-tour-got-it'))

    expect(dismissSpy).toHaveBeenCalledWith('welcome')
    expect(dismissSpy).toHaveBeenCalledTimes(1)
    expect(localStorage.getItem(STORAGE_KEYS.WELCOME_SEEN)).toBe('1')
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('X close button calls api.dismissTour("welcome")', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<WelcomeTour onClose={onClose} />)

    await user.click(screen.getByTestId('welcome-tour-close'))

    expect(dismissSpy).toHaveBeenCalledWith('welcome')
    expect(localStorage.getItem(STORAGE_KEYS.WELCOME_SEEN)).toBe('1')
  })

  it('Escape keypress calls api.dismissTour("welcome")', () => {
    const onClose = vi.fn()
    render(<WelcomeTour onClose={onClose} />)

    fireEvent.keyDown(document, { key: 'Escape' })

    expect(dismissSpy).toHaveBeenCalledWith('welcome')
    expect(localStorage.getItem(STORAGE_KEYS.WELCOME_SEEN)).toBe('1')
  })

  it('backdrop click calls api.dismissTour("welcome")', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<WelcomeTour onClose={onClose} />)

    const dialog = screen.getByRole('dialog')
    const backdrop = dialog.parentElement
    expect(backdrop).toBeTruthy()
    fireEvent.click(backdrop!, { target: backdrop } as unknown as MouseEvent)

    await waitFor(() => {
      expect(dismissSpy).toHaveBeenCalledWith('welcome')
    })
  })

  it('still calls onClose when the server call fails (network blip)', async () => {
    // The server call is fire-and-forget. A 500 from the server
    // must NOT crash the UI or block the dismissal — the
    // localStorage write still hides the tour for this device.
    dismissSpy.mockRejectedValue(new Error('network down'))
    const onClose = vi.fn()
    const user = userEvent.setup()
    const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})

    render(<WelcomeTour onClose={onClose} />)
    await user.click(screen.getByTestId('welcome-tour-got-it'))

    // The dialog unmounts (onClose fires) even though the server
    // POST failed.
    expect(onClose).toHaveBeenCalledTimes(1)
    // The localStorage flag is still set — the user shouldn't see
    // the tour again on this device.
    expect(localStorage.getItem(STORAGE_KEYS.WELCOME_SEEN)).toBe('1')
    // The failure was logged so devs can notice in the console.
    expect(consoleSpy).toHaveBeenCalled()
    consoleSpy.mockRestore()
  })
})
