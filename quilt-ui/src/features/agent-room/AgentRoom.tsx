// ─── AgentRoom ──────────────────────────────────────────────────────
//
// The CG-5 V1 panel: list + spawn + cancel of agent runs.
// Renders the four states (loading / error / empty / with-data),
// the spawn form, and a list of `AgentCard`s. Polls every 5s
// while at least one agent is `Queued` or `Running`.
//
// This component is mounted into `CognitivePanels` behind
// the `agent-room` panel flag. It is also independently
// mountable: no required props.

import { useCallback, useEffect, useRef, useState } from 'react'
import { Bot, Loader2, RefreshCw } from 'lucide-react'
import { api } from '@core/api-client'
import type { AgentDto, AgentListResponse, SpawnAgentRequest } from '@shared/types/api'
import { AgentCard } from './AgentCard'
import { AgentSpawnForm } from './AgentSpawnForm'

const POLL_INTERVAL_MS = 5_000
const MAX_VISIBLE_CARDS = 50

const ACTIVE_STATUSES = new Set(['Queued', 'Running'])

interface AgentRoomState {
  data: AgentListResponse | null
  loading: boolean
  refreshing: boolean
  error: string | null
  spawnError: string | null
  submitting: boolean
  availableTypes: string[]
}

function initialState(): AgentRoomState {
  return {
    data: null,
    loading: true,
    refreshing: false,
    error: null,
    spawnError: null,
    submitting: false,
    availableTypes: ['decay-annotator'],
  }
}

export function AgentRoom() {
  const [state, setState] = useState<AgentRoomState>(initialState)
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const cancelledIdsRef = useRef<Set<string>>(new Set())

  const load = useCallback(async (isRefresh: boolean) => {
    if (isRefresh) setState((s) => ({ ...s, refreshing: true }))
    else setState((s) => ({ ...s, loading: true, error: null }))
    try {
      const data = await api.listAgents()
      setState((s) => ({ ...s, data, loading: false, refreshing: false, error: null }))
    } catch (err) {
      setState((s) => ({
        ...s,
        loading: false,
        refreshing: false,
        error: err instanceof Error ? err.message : 'Failed to load agents',
      }))
    }
  }, [])

  // Polling loop: 5s while at least one agent is active.
  useEffect(() => {
    function stop() {
      if (pollRef.current !== null) {
        clearInterval(pollRef.current)
        pollRef.current = null
      }
    }
    function start() {
      if (pollRef.current !== null) return
      pollRef.current = setInterval(() => {
        void load(true)
      }, POLL_INTERVAL_MS)
    }
    const hasActive =
      (state.data?.agents ?? []).some((a) => ACTIVE_STATUSES.has(a.status)) ?? false
    if (hasActive) {
      start()
    } else {
      stop()
    }
    return stop
  }, [state.data, load])

  // Initial fetch + cleanup on unmount.
  useEffect(() => {
    void load(false)
    return () => {
      if (pollRef.current !== null) {
        clearInterval(pollRef.current)
        pollRef.current = null
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Spawn handler. Optimistically inserts the Queued agent
  // at the top of the list.
  const handleSpawn = useCallback(
    async (req: SpawnAgentRequest) => {
      setState((s) => ({ ...s, submitting: true, spawnError: null }))
      try {
        const dto = await api.spawnAgent(req)
        setState((s) => {
          const data: AgentListResponse = s.data ?? { agents: [], total: 0 }
          const agents = [dto, ...data.agents.filter((a) => a.id !== dto.id)]
          return {
            ...s,
            submitting: false,
            data: { agents, total: data.total + 1 },
          }
        })
      } catch (err) {
        setState((s) => ({
          ...s,
          submitting: false,
          spawnError: err instanceof Error ? err.message : 'Failed to spawn agent',
        }))
      }
    },
    [],
  )

  // Cancel handler. Optimistic — flips the card to
  // Cancelled locally before the server confirms.
  const handleCancel = useCallback(async (id: string) => {
    cancelledIdsRef.current.add(id)
    setState((s) => {
      if (!s.data) return s
      const agents = s.data.agents.map((a) =>
        a.id === id && ACTIVE_STATUSES.has(a.status)
          ? { ...a, status: 'Cancelled' as const, completedAt: new Date().toISOString() }
          : a,
      )
      return { ...s, data: { ...s.data, agents } }
    })
    try {
      const dto = await api.cancelAgent(id)
      setState((s) => {
        if (!s.data) return s
        const agents = s.data.agents.map((a) => (a.id === id ? dto : a))
        return { ...s, data: { ...s.data, agents } }
      })
    } catch (err) {
      // Revert optimistic update.
      cancelledIdsRef.current.delete(id)
      setState((s) => {
        if (!s.data) return s
        const agents = s.data.agents.map((a) =>
          a.id === id
            ? { ...a, status: 'Running' as const, completedAt: null }
            : a,
        )
        return {
          ...s,
          data: { ...s.data, agents },
          error: err instanceof Error ? err.message : 'Failed to cancel agent',
        }
      })
    }
  }, [])

  const agents = state.data?.agents ?? []
  const visible = agents.slice(0, MAX_VISIBLE_CARDS)
  const overflow = agents.length - visible.length
  const activeCount = agents.filter((a) => ACTIVE_STATUSES.has(a.status)).length

  return (
    <div
      data-testid="agent-room"
      role="region"
      aria-label="Agent Room"
      style={{
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-lg)',
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: 'var(--space-3)',
          borderBottom: '1px solid var(--color-border)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            fontWeight: 600,
            fontSize: '14px',
          }}
        >
          <Bot size={16} color="var(--color-accent)" />
          <span>Agent Room</span>
          {state.data && (
            <span
              data-testid="agent-room-active-count"
              style={{ color: 'var(--color-text-muted)', fontSize: '11px' }}
            >
              {activeCount} active
            </span>
          )}
        </div>
        <button
          data-testid="agent-room-refresh"
          aria-label="Refresh agents"
          onClick={() => void load(true)}
          disabled={state.refreshing}
          style={{
            background: 'none',
            border: 'none',
            cursor: state.refreshing ? 'default' : 'pointer',
            color: 'var(--color-text-muted)',
            display: 'inline-flex',
            alignItems: 'center',
            padding: '4px',
            borderRadius: 'var(--radius-sm)',
          }}
        >
          <RefreshCw
            size={14}
            style={{
              animation: state.refreshing ? 'spin 1s linear infinite' : 'none',
            }}
          />
        </button>
      </div>

      {/* Spawn form (always present so the user can spawn
          from a cold state without scrolling). */}
      <AgentSpawnForm
        availableTypes={state.availableTypes}
        onSubmit={handleSpawn}
        submitting={state.submitting}
        error={state.spawnError}
      />

      {/* States */}
      {state.loading && (
        <div
          data-testid="agent-room-loading"
          style={{ padding: 'var(--space-3)', textAlign: 'center' }}
        >
          <Loader2
            size={16}
            color="var(--color-text-muted)"
            style={{ animation: 'spin 1s linear infinite' }}
          />
          <div
            style={{
              color: 'var(--color-text-muted)',
              fontSize: '12px',
              marginTop: 'var(--space-1)',
            }}
          >
            Loading agents…
          </div>
        </div>
      )}

      {!state.loading && state.error && (
        <div
          data-testid="agent-room-error"
          style={{
            padding: 'var(--space-3)',
            color: 'var(--color-danger, #c0392b)',
            fontSize: '12px',
          }}
        >
          {state.error}
        </div>
      )}

      {!state.loading && !state.error && agents.length === 0 && (
        <div
          data-testid="agent-room-empty"
          style={{
            padding: 'var(--space-3)',
            color: 'var(--color-text-muted)',
            fontSize: '12px',
            fontStyle: 'italic',
            textAlign: 'center',
          }}
        >
          No agents spawned yet — pick a type above and click Spawn.
        </div>
      )}

      {!state.loading && !state.error && agents.length > 0 && (
        <>
          <ul
            data-testid="agent-room-list"
            role="list"
            style={{ listStyle: 'none', margin: 0, padding: 0 }}
          >
            {visible.map((agent) => (
              <li key={agent.id}>
                <AgentCard agent={agent} onCancel={handleCancel} />
              </li>
            ))}
          </ul>
          {overflow > 0 && (
            <div
              data-testid="agent-room-overflow"
              style={{
                padding: 'var(--space-2)',
                color: 'var(--color-text-muted)',
                fontSize: '10px',
                textAlign: 'center',
                borderTop: '1px solid var(--color-border)',
              }}
            >
              …and {overflow} more
            </div>
          )}
        </>
      )}
    </div>
  )
}
