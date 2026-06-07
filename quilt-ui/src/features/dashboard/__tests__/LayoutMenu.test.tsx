// ─── LayoutMenu.test.tsx — dashboard layout dropdown ─────────────
//
// The LayoutMenu is the user-facing surface of the dashboard
// feature. It renders a button that opens a dropdown with three
// preset buttons and one checkbox per panel. The tests pin the
// interactions:
//   - trigger opens / closes the menu
//   - clicking a preset button applies the preset
//   - clicking a checkbox toggles the panel
//   - outside click + Escape close the menu

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { type ReactNode } from 'react'
import { LayoutMenu } from '../LayoutMenu'
import {
  PanelVisibilityProvider,
  usePanelVisibility,
} from '../PanelVisibilityContext'
import { getPreset } from '../presets'

/** A harness that lets a test observe context state via the DOM. */
function Harness() {
  const { visiblePanels, lastAppliedPreset } = usePanelVisibility()
  return (
    <div>
      <LayoutMenu />
      <output data-testid="visible-panels">
        {Array.from(visiblePanels).sort().join(',')}
      </output>
      <output data-testid="last-preset">
        {lastAppliedPreset ?? 'none'}
      </output>
    </div>
  )
}

function renderWithProvider(ui: ReactNode) {
  return render(<PanelVisibilityProvider>{ui}</PanelVisibilityProvider>)
}

beforeEach(() => {
  localStorage.clear()
})

describe('LayoutMenu — trigger', () => {
  it('renders a layout button in the closed state', () => {
    renderWithProvider(<Harness />)
    const trigger = screen.getByTestId('layout-menu-trigger')
    expect(trigger).toBeInTheDocument()
    expect(trigger).toHaveAttribute('aria-expanded', 'false')
  })

  it('opens the menu when the trigger is clicked and sets aria-expanded', async () => {
    const user = userEvent.setup()
    renderWithProvider(<Harness />)
    await user.click(screen.getByTestId('layout-menu-trigger'))
    expect(screen.getByTestId('layout-menu-trigger')).toHaveAttribute('aria-expanded', 'true')
    // The dropdown is in the DOM after opening.
    expect(screen.getByTestId('layout-menu')).toBeInTheDocument()
  })

  it('closes the menu when the trigger is clicked a second time', async () => {
    const user = userEvent.setup()
    renderWithProvider(<Harness />)
    await user.click(screen.getByTestId('layout-menu-trigger'))
    await user.click(screen.getByTestId('layout-menu-trigger'))
    expect(screen.queryByTestId('layout-menu')).not.toBeInTheDocument()
  })

  it('closes the menu on Escape', async () => {
    const user = userEvent.setup()
    renderWithProvider(<Harness />)
    await user.click(screen.getByTestId('layout-menu-trigger'))
    expect(screen.getByTestId('layout-menu')).toBeInTheDocument()
    fireEscape()
    expect(screen.queryByTestId('layout-menu')).not.toBeInTheDocument()
  })

  it('closes the menu on outside click', async () => {
    const user = userEvent.setup()
    renderWithProvider(
      <div>
        <Harness />
        <button data-testid="outside">Outside</button>
      </div>,
    )
    await user.click(screen.getByTestId('layout-menu-trigger'))
    expect(screen.getByTestId('layout-menu')).toBeInTheDocument()
    await user.click(screen.getByTestId('outside'))
    expect(screen.queryByTestId('layout-menu')).not.toBeInTheDocument()
  })
})

describe('LayoutMenu — preset buttons', () => {
  it('renders one button per preset', async () => {
    const user = userEvent.setup()
    renderWithProvider(<Harness />)
    await user.click(screen.getByTestId('layout-menu-trigger'))
    expect(screen.getByTestId('layout-preset-default')).toBeInTheDocument()
    expect(screen.getByTestId('layout-preset-focus')).toBeInTheDocument()
    expect(screen.getByTestId('layout-preset-review')).toBeInTheDocument()
  })

  it('clicking a preset button applies its panel set and updates the visible-panels output', async () => {
    const user = userEvent.setup()
    renderWithProvider(<Harness />)
    await user.click(screen.getByTestId('layout-menu-trigger'))
    await user.click(screen.getByTestId('layout-preset-focus'))
    const out = screen.getByTestId('visible-panels')
    expect(out.textContent).toBe(
      Array.from(getPreset('focus')).sort().join(','),
    )
  })

  it('records the most recently applied preset name in lastAppliedPreset', async () => {
    const user = userEvent.setup()
    renderWithProvider(<Harness />)
    await user.click(screen.getByTestId('layout-menu-trigger'))
    await user.click(screen.getByTestId('layout-preset-review'))
    expect(screen.getByTestId('last-preset').textContent).toBe('review')
  })

  it('closes the menu after a preset is applied', async () => {
    const user = userEvent.setup()
    renderWithProvider(<Harness />)
    await user.click(screen.getByTestId('layout-menu-trigger'))
    await user.click(screen.getByTestId('layout-preset-default'))
    expect(screen.queryByTestId('layout-menu')).not.toBeInTheDocument()
  })
})

describe('LayoutMenu — panel checkboxes', () => {
  it('renders a checkbox for every known panel', async () => {
    const user = userEvent.setup()
    renderWithProvider(<Harness />)
    await user.click(screen.getByTestId('layout-menu-trigger'))
    for (const panel of ['sidebar', 'backlinks', 'agent-activity', 'outline']) {
      expect(
        screen.getByTestId(`layout-toggle-${panel}`),
      ).toBeInTheDocument()
    }
  })

  it('clicking a checkbox toggles that panel only', async () => {
    const user = userEvent.setup()
    renderWithProvider(<Harness />)
    await user.click(screen.getByTestId('layout-menu-trigger'))

    const agentToggle = screen.getByTestId('layout-toggle-agent-activity')
    const beforeHas = getPreset('default').has('agent-activity')
    await user.click(agentToggle)

    const out = screen.getByTestId('visible-panels')
    const expectedSet = new Set(getPreset('default'))
    if (beforeHas) expectedSet.delete('agent-activity')
    else expectedSet.add('agent-activity')
    expect(out.textContent).toBe(Array.from(expectedSet).sort().join(','))
  })

  it('keeps the menu open after a checkbox click (so the user can toggle several)', async () => {
    const user = userEvent.setup()
    renderWithProvider(<Harness />)
    await user.click(screen.getByTestId('layout-menu-trigger'))
    await user.click(screen.getByTestId('layout-toggle-outline'))
    expect(screen.getByTestId('layout-menu')).toBeInTheDocument()
  })
})

// ─── helpers ──────────────────────────────────────────────────────

function fireEscape() {
  act(() => {
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
  })
}
