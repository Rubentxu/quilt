import type { BlockRenderer } from './types'

export const AGENT_RUN_STATUSES = ['Queued', 'Running', 'Completed', 'Failed', 'Cancelled'] as const
export type AgentRunStatus = (typeof AGENT_RUN_STATUSES)[number]

const AGENT_RUN_STATUS_STYLES: Record<AgentRunStatus, { bg: string; text: string }> = {
  Queued: { bg: 'var(--color-text-muted)', text: '#fff' },
  Running: { bg: 'var(--color-info)', text: '#fff' },
  Completed: { bg: 'var(--color-success)', text: '#fff' },
  Failed: { bg: 'var(--color-danger)', text: '#fff' },
  Cancelled: { bg: 'var(--color-text-disabled)', text: '#fff' },
}

function readProperty(block: any, key: string): string | null {
  const prop = block.properties?.find((p: any) => p.key === key)
  if (!prop || prop.value == null) return null
  return String(prop.value)
}

export const AgentRunRenderer: BlockRenderer = {
  id: 'agent-run',
  priority: 12,

  match(block, strategy) {
    return strategy === 'agent-run'
  },

  renderBeforeContent(ctx) {
    const agentName = readProperty(ctx.block, 'agent')
    const agentModel = readProperty(ctx.block, 'model')
    const runStatusRaw = readProperty(ctx.block, 'run-status')
    const runStatus: AgentRunStatus | null =
      runStatusRaw && (AGENT_RUN_STATUSES as readonly string[]).includes(runStatusRaw)
        ? (runStatusRaw as AgentRunStatus)
        : null
    const startedAt = readProperty(ctx.block, 'started-at')
    const runError = readProperty(ctx.block, 'error')

    if (!agentName && !agentModel && !runStatus && !startedAt) return null

    return (
      <div
        data-testid="agent-run-header"
        aria-label="Agent run"
        title={
          runError
            ? `Agent run (${runStatus ?? 'unknown'}): ${runError}`
            : runStatus
              ? `Agent run (${runStatus})`
              : 'Agent run'
        }
        style={{
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          alignSelf: 'center',
          flexWrap: 'wrap',
          maxWidth: '100%',
        }}
      >
        {agentName && (
          <span
            data-testid="agent-run-agent"
            style={{
              fontSize: '11px', fontWeight: 600,
              padding: '2px 8px', borderRadius: 'var(--radius-pill)',
              background: 'var(--color-accent-subtle, rgba(99, 102, 241, 0.12))',
              color: 'var(--color-accent)',
              lineHeight: 1.4, display: 'inline-flex',
              alignItems: 'center', gap: '4px', whiteSpace: 'nowrap',
            }}
          >
            <span aria-hidden="true">🤖</span>
            {agentName}
          </span>
        )}
        {agentModel && (
          <span data-testid="agent-run-model" style={{ fontSize: '11px', fontWeight: 500, color: 'var(--color-text-muted)', whiteSpace: 'nowrap' }}>
            {agentModel}
          </span>
        )}
        {runStatus && (
          <span
            data-testid="agent-run-status"
            style={{
              fontSize: '11px', fontWeight: 600,
              padding: '2px 8px', borderRadius: 'var(--radius-pill)',
              background: AGENT_RUN_STATUS_STYLES[runStatus].bg,
              color: AGENT_RUN_STATUS_STYLES[runStatus].text,
              lineHeight: 1.4, letterSpacing: '0.01em', whiteSpace: 'nowrap',
            }}
          >
            {runStatus.toUpperCase()}
          </span>
        )}
        {startedAt && (
          <span data-testid="agent-run-started-at" title={`Started at ${startedAt}`} style={{ fontSize: '11px', fontWeight: 400, color: 'var(--color-text-muted)', whiteSpace: 'nowrap' }}>
            {startedAt}
          </span>
        )}
      </div>
    )
  },
}
