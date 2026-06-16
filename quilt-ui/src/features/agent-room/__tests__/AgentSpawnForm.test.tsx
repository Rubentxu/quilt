// Component tests for AgentSpawnForm (CG-5 V1).

import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AgentSpawnForm } from '../AgentSpawnForm'
import type { SpawnAgentRequest } from '@shared/types/api'

describe('AgentSpawnForm', () => {
  it('submits with typed context page', async () => {
    const user = userEvent.setup()
    const onSubmit = vi.fn().mockResolvedValue(undefined)
    render(
      <AgentSpawnForm availableTypes={['decay-annotator']} onSubmit={onSubmit} />,
    )
    await user.type(screen.getByTestId('agent-room-spawn-context'), 'p/x')
    await user.click(screen.getByTestId('agent-room-spawn-submit'))
    expect(onSubmit).toHaveBeenCalledWith({
      agentType: 'decay-annotator',
      contextPage: 'p/x',
    })
  })

  it('submits without contextPage when input is empty', async () => {
    const user = userEvent.setup()
    const onSubmit = vi.fn().mockResolvedValue(undefined)
    render(
      <AgentSpawnForm availableTypes={['decay-annotator']} onSubmit={onSubmit} />,
    )
    await user.click(screen.getByTestId('agent-room-spawn-submit'))
    const arg = onSubmit.mock.calls[0][0] as SpawnAgentRequest
    expect(arg.agentType).toBe('decay-annotator')
    expect(arg.contextPage).toBeUndefined()
  })

  it('disables the submit button while submitting', () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined)
    render(
      <AgentSpawnForm
        availableTypes={['decay-annotator']}
        onSubmit={onSubmit}
        submitting={true}
      />,
    )
    expect(screen.getByTestId('agent-room-spawn-submit')).toBeDisabled()
  })

  it('shows an inline error from the parent', () => {
    render(
      <AgentSpawnForm
        availableTypes={['decay-annotator']}
        onSubmit={vi.fn().mockResolvedValue(undefined)}
        error="Unknown agent type"
      />,
    )
    expect(screen.getByTestId('agent-room-spawn-error').textContent).toBe(
      'Unknown agent type',
    )
  })
})
