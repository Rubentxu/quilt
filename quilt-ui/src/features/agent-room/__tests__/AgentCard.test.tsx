// Component tests for AgentCard (CG-5 V1).

import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AgentCard } from '../AgentCard'
import type { AgentDto } from '@shared/types/api'

function makeAgent(overrides: Partial<AgentDto> = {}): AgentDto {
  return {
    id: 'agent-1',
    agentType: 'decay-annotator',
    model: 'decay-annotator-v1',
    status: 'Queued',
    contextPage: null,
    summary: null,
    blocksModified: 0,
    startedAt: null,
    completedAt: null,
    error: null,
    ...overrides,
  }
}

describe('AgentCard', () => {
  it('renders Queued status with cancel button visible', () => {
    const onCancel = vi.fn()
    render(<AgentCard agent={makeAgent({ status: 'Queued' })} onCancel={onCancel} />)
    expect(screen.getByTestId('agent-card-status-agent-1').textContent).toBe('QUEUED')
    expect(screen.getByTestId('agent-card-cancel-agent-1')).toBeInTheDocument()
  })

  it('renders Running status with cancel button visible', () => {
    const onCancel = vi.fn()
    render(
      <AgentCard
        agent={makeAgent({
          status: 'Running',
          startedAt: '2026-06-16T10:00:00Z',
        })}
        onCancel={onCancel}
      />,
    )
    expect(screen.getByTestId('agent-card-status-agent-1').textContent).toBe('RUNNING')
    expect(screen.getByTestId('agent-card-cancel-agent-1')).toBeInTheDocument()
  })

  it('renders Completed status with no cancel button and shows summary', () => {
    render(
      <AgentCard
        agent={makeAgent({
          status: 'Completed',
          summary: 'Found 3 stale blocks',
          blocksModified: 3,
          startedAt: '2026-06-16T10:00:00Z',
          completedAt: '2026-06-16T10:00:30Z',
        })}
      />,
    )
    expect(screen.getByTestId('agent-card-status-agent-1').textContent).toBe('COMPLETED')
    expect(screen.getByTestId('agent-card-summary-agent-1').textContent).toBe(
      'Found 3 stale blocks',
    )
    expect(screen.queryByTestId('agent-card-cancel-agent-1')).not.toBeInTheDocument()
  })

  it('renders Failed status with no cancel button and shows error', () => {
    render(
      <AgentCard
        agent={makeAgent({
          status: 'Failed',
          error: 'Agent run exceeded 5-minute timeout',
        })}
      />,
    )
    expect(screen.getByTestId('agent-card-status-agent-1').textContent).toBe('FAILED')
    expect(screen.getByTestId('agent-card-error-agent-1').textContent).toBe(
      'Agent run exceeded 5-minute timeout',
    )
    expect(screen.queryByTestId('agent-card-cancel-agent-1')).not.toBeInTheDocument()
  })

  it('calls onCancel when the cancel button is clicked', async () => {
    const user = userEvent.setup()
    const onCancel = vi.fn().mockResolvedValue(undefined)
    render(<AgentCard agent={makeAgent({ status: 'Running' })} onCancel={onCancel} />)
    await user.click(screen.getByTestId('agent-card-cancel-agent-1'))
    expect(onCancel).toHaveBeenCalledWith('agent-1')
  })
})
