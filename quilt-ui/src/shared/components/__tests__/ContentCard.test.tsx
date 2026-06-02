import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi } from 'vitest'
import { ContentCard } from '../ContentCard'

describe('ContentCard — DESIGN.md §9.8', () => {
  it('renders the title and children by default', () => {
    render(
      <ContentCard title="Documentación Pipelines Correos">
        <p>Contenido del card</p>
      </ContentCard>,
    )
    expect(screen.getByText('Documentación Pipelines Correos')).toBeInTheDocument()
    expect(screen.getByText('Contenido del card')).toBeInTheDocument()
  })

  it('starts collapsed when defaultCollapsed is true', () => {
    render(
      <ContentCard title="Sección" defaultCollapsed>
        <p>No debería verse</p>
      </ContentCard>,
    )
    expect(screen.queryByText('No debería verse')).not.toBeInTheDocument()
  })

  it('expands when clicking the expand button', async () => {
    const user = userEvent.setup()
    render(
      <ContentCard title="Sección" defaultCollapsed>
        <p>Aparece al expandir</p>
      </ContentCard>,
    )
    await user.click(screen.getByRole('button', { name: 'Expand section' }))
    expect(screen.getByText('Aparece al expandir')).toBeInTheDocument()
  })
})
