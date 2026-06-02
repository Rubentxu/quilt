import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi } from 'vitest'
import { ReferenceCard } from '../ReferenceCard'

describe('ReferenceCard — DESIGN.md §9.7', () => {
  it('renders title and meta values', () => {
    render(
      <ReferenceCard
        title="Incidencias falta credenciales huella_app y huella_own"
        metas={[
          { key: 'dda-relacionada', value: 'DDA Huella v1.0.0' },
          { key: 'type', value: 'Incidencia' },
          { key: 'fecha-creacion', value: '26-05-2026' },
        ]}
        href="/page/incidencias"
      />
    )
    expect(
      screen.getByText('Incidencias falta credenciales huella_app y huella_own'),
    ).toBeInTheDocument()
    expect(screen.getByText('DDA Huella v1.0.0')).toBeInTheDocument()
    expect(screen.getByText('Incidencia')).toBeInTheDocument()
    expect(screen.getByText('26-05-2026')).toBeInTheDocument()
  })

  it('renders a link when href is provided', () => {
    render(
      <ReferenceCard title="Reference" href="/page/foo" />
    )
    const link = screen.getByText('Reference')
    expect(link.tagName).toBe('A')
    expect(link).toHaveAttribute('href', '/page/foo')
  })

  it('toggles the menu when More actions is clicked', async () => {
    const user = userEvent.setup()
    render(<ReferenceCard title="Ref" />)
    await user.click(screen.getByRole('button', { name: 'More actions' }))
    expect(screen.getByRole('menu')).toBeInTheDocument()
    expect(screen.getByRole('menuitem', { name: /Copy link/ })).toBeInTheDocument()
  })
})
