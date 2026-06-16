// Component tests for AgentRoom (CG-5 V1).

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AgentRoom } from '../AgentRoom'
import type { AgentDto, AgentListResponse } from '@shared/types/api'

// Mock the api client. The mock is hoisted by Vitest.
vi.mock('@core/api-client', () => ({
  api: {
    listAgents: vi.fn(),
    spawnAgent: vi.fn(),
    cancelAgent: vi.fn(),
  },
}))

import { api } from '@core/api-client'

const listAgents = api.listAgents as unknown as ReturnType<typeof vi.fn>
const spawnAgent = api.spawnAgent as unknown as ReturnType<typeof vi.fn>
const cancelAgent = api.cancelAgent as unknown as ReturnType<typeof vi.fn>

function makeAgent(overrides: Partial<AgentDto> = {}): AgentDto {
  return {
    id: 'agent-1',
    agentType: 'decay-annotator',
    model: 'decay-annotator-v1',
    status: 'Completed',
    contextPage: null,
    summary: 'ok',
    blocksModified: 0,
    startedAt: '2026-06-16T10:00:00Z',
    completedAt: '2026-06-16T10:00:30Z',
    error: null,
    ...overrides,
  }
}

function listResponse(agents: AgentDto[], total?: number): AgentListResponse {
  return { agents, total: total ?? agents.length }
}

beforeEach(() => {
  listAgents.mockReset()
  spawnAgent.mockReset()
  cancelAgent.mockReset()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('AgentRoom', () => {
  it('shows the loading state on initial mount', () => {
    listAgents.mockReturnValue(new Promise(() => {})) // never resolves
    render(<AgentRoom />)
    expect(screen.getByTestId('agent-room-loading')).toBeInTheDocument()
  })

  it('shows the error state when listAgents rejects', async () => {
    listAgents.mockRejectedValue(new Error('Network error'))
    render(<AgentRoom />)
    await waitFor(() => {
      expect(screen.getByTestId('agent-room-error')).toBeInTheDocument()
    })
    expect(screen.getByTestId('agent-room-error').textContent).toBe('Network error')
  })

  it('shows the empty state when the registry is empty', async () => {
    listAgents.mockResolvedValue(listResponse([]))
    render(<AgentRoom />)
    await waitFor(() => {
      expect(screen.getByTestId('agent-room-empty')).toBeInTheDocument()
    })
    expect(screen.queryByTestId('agent-room-list')).not.toBeInTheDocument()
  })

  it('shows the list state with one card per agent', async () => {
    listAgents.mockResolvedValue(
      listResponse([
        makeAgent({ id: 'a' }),
        makeAgent({ id: 'b', status: 'Running' }),
      ]),
    )
    render(<AgentRoom />)
    await waitFor(() => {
      expect(screen.getByTestId('agent-room-list')).toBeInTheDocument()
    })
    expect(screen.getByTestId('agent-card-a')).toBeInTheDocument()
    expect(screen.getByTestId('agent-card-b')).toBeInTheDocument()
    expect(screen.getByTestId('agent-room-active-count').textContent).toContain(
      '1 active',
    )
  })

  it('refresh button re-fetches with the refresh indicator', async () => {
    const user = userEvent.setup()
    listAgents.mockResolvedValue(listResponse([makeAgent({ id: 'a' })]))
    render(<AgentRoom />)
    await waitFor(() => {
      expect(screen.getByTestId('agent-room-list')).toBeInTheDocument()
    })
    expect(listAgents).toHaveBeenCalledTimes(1)
    await user.click(screen.getByTestId('agent-room-refresh'))
    expect(listAgents).toHaveBeenCalledTimes(2)
  })

  it('optimistically inserts a Queued agent on spawn', async () => {
    const user = userEvent.setup()
    listAgents.mockResolvedValue(listResponse([]))
    spawnAgent.mockImplementation(async (req) => ({
      id: 'new-1',
      agentType: req.agentType,
      model: null,
      status: 'Queued' as const,
      contextPage: req.contextPage ?? null,
      summary: null,
      blocksModified: 0,
      startedAt: null,
      completedAt: null,
      error: null,
    }))
    render(<AgentRoom />)
    await waitFor(() => {
      expect(screen.getByTestId('agent-room-empty')).toBeInTheDocument()
    })
    await user.click(screen.getByTestId('agent-room-spawn-submit'))
    await waitFor(() => {
      expect(screen.getByTestId('agent-card-new-1')).toBeInTheDocument()
    })
  })

  it('optimistically flips a Running card to Cancelled on cancel', async () => {
    const user = userEvent.setup()
    listAgents.mockResolvedValue(
      listResponse([makeAgent({ id: 'a', status: 'Running' })]),
    )
    cancelAgent.mockResolvedValue(
      makeAgent({ id: 'a', status: 'Cancelled', completedAt: '2026-06-16T10:01:00Z' }),
    )
    render(<AgentRoom />)
    await waitFor(() => {
      expect(screen.getByTestId('agent-room-list')).toBeInTheDocument()
    })
    await user.click(screen.getByTestId('agent-card-cancel-a'))
    await waitFor(() => {
      const card = screen.getByTestId('agent-card-a')
      expect(card.getAttribute('data-agent-status')).toBe('Cancelled')
    })
    expect(cancelAgent).toHaveBeenCalledWith('a')
  })

  it('does not poll when no agent is active (initial fetch only)', async () => {
    // No agents are Queued or Running, so the polling
    // interval is never scheduled.
    listAgents.mockResolvedValue(
      listResponse([makeAgent({ id: 'a', status: 'Completed' })]),
    )
    render(<AgentRoom />)
    await waitFor(() => {
      expect(listAgents).toHaveBeenCalledTimes(1)
    })
    // Wait a bit and verify no second call.
    await new Promise((r) => setTimeout(r, 50))
    expect(listAgents).toHaveBeenCalledTimes(1)
  })
})
