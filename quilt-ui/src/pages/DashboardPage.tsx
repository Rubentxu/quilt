/**
 * DashboardPage — replaces the empty HomePage with a useful overview.
 *
 * Shows:
 * - Quick stats (total pages, blocks, templates)
 * - Recent activity (agent-authored blocks)
 * - Quick actions (new page, search, kanban, table)
 * - Template list
 */

import { useEffect, useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'react-hot-toast'
import { api } from '@core/api-client'
import { AgentActivityFeed } from '@features/cognitive/AgentActivityFeed'

interface Stats {
  pages: number
  blocks: number
  templates: number
  journals: number
}

export function DashboardPage() {
  const [stats, setStats] = useState<Stats | null>(null)
  const [loading, setLoading] = useState(true)
  const navigate = useNavigate()

  useEffect(() => {
    Promise.all([
      api.listPages(),
      api.listTemplates(),
    ])
      .then(([pages, templates]) => {
        const journals = pages.filter(p => p.journal).length
        // block_count is not in the Page type; skip for now
        setStats({
          pages: pages.length - journals,
          blocks: 0, // block_count not available in Page type; TODO: fetch from API
          templates: templates.length,
          journals,
        })
      })
      .catch(() => toast.error('Failed to load dashboard stats'))
      .finally(() => setLoading(false))
  }, [])

  if (loading) {
    return (
      <div style={{ padding: 'var(--space-8)', textAlign: 'center', color: 'var(--color-text-muted)' }}>
        Loading dashboard…
      </div>
    )
  }

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        padding: 'var(--space-6) var(--space-5)',
        gap: 'var(--space-6)',
        maxWidth: '900px',
        margin: '0 auto',
      }}
    >
      {/* Header */}
      <div>
        <h1
          style={{
            fontSize: '24px',
            fontWeight: 700,
            color: 'var(--color-text-primary)',
            margin: '0 0 var(--space-1) 0',
          }}
        >
          Quilt Workspace
        </h1>
        <p style={{ fontSize: '14px', color: 'var(--color-text-muted)', margin: 0 }}>
          Your AI-first knowledge graph
        </p>
      </div>

      {/* Stats cards */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
          gap: 'var(--space-4)',
        }}
      >
        <StatCard
          label="Pages"
          value={stats?.pages ?? 0}
          onClick={() => navigate({ to: '/pages' })}
        />
        <StatCard
          label="Blocks"
          value={stats?.blocks ?? 0}
          onClick={() => navigate({ to: '/table' })}
        />
        <StatCard
          label="Templates"
          value={stats?.templates ?? 0}
          onClick={() => navigate({ to: '/pages' })}
        />
        <StatCard
          label="Journals"
          value={stats?.journals ?? 0}
          onClick={() => navigate({ to: `/journal/${new Date().toISOString().split('T')[0]}` })}
        />
      </div>

      {/* Quick actions */}
      <div>
        <h2
          style={{
            fontSize: '16px',
            fontWeight: 600,
            color: 'var(--color-text-primary)',
            margin: '0 0 var(--space-3) 0',
          }}
        >
          Quick Actions
        </h2>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-3)' }}>
          <QuickAction
            label="New Page"
            description="Create a blank page"
            onClick={() => {
              const name = window.prompt('Page name:')
              if (name?.trim()) {
                api.createPage({ name: name.trim().toLowerCase() })
                  .then(p => navigate({ to: `/page/${encodeURIComponent(p.name)}` }))
                  .catch(err => toast.error(`Failed: ${err.message}`))
              }
            }}
          />
          <QuickAction
            label="Search"
            description="Find blocks and pages"
            onClick={() => {
              const searchInput = document.querySelector('[data-testid="sidebar-search-input"]') as HTMLInputElement
              searchInput?.focus()
            }}
          />
          <QuickAction
            label="Table View"
            description="Query and filter blocks"
            onClick={() => navigate({ to: '/table' })}
          />
          <QuickAction
            label="Kanban Board"
            description="Visualize by property"
            onClick={() => navigate({ to: '/kanban' })}
          />
          <QuickAction
            label="Graph View"
            description="Explore connections"
            onClick={() => navigate({ to: '/graph' })}
          />
        </div>
      </div>

      {/* Agent activity */}
      <div>
        <h2
          style={{
            fontSize: '16px',
            fontWeight: 600,
            color: 'var(--color-text-primary)',
            margin: '0 0 var(--space-3) 0',
          }}
        >
          Agent Activity
        </h2>
        <div
          style={{
            background: 'var(--color-surface)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-lg)',
            padding: 'var(--space-4)',
          }}
        >
          <AgentActivityFeed maxItems={10} />
        </div>
      </div>
    </div>
  )
}

function StatCard({ label, value, onClick }: { label: string; value: number; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'flex-start',
        padding: 'var(--space-4)',
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-lg)',
        cursor: 'pointer',
        transition: 'border-color var(--motion-fast)',
        textAlign: 'left',
      }}
    >
      <span
        style={{
          fontSize: '28px',
          fontWeight: 700,
          color: 'var(--color-primary)',
          lineHeight: 1,
        }}
      >
        {value}
      </span>
      <span
        style={{
          fontSize: '13px',
          color: 'var(--color-text-muted)',
          marginTop: 'var(--space-1)',
        }}
      >
        {label}
      </span>
    </button>
  )
}

function QuickAction({ label, description, onClick }: { label: string; description: string; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'flex-start',
        padding: 'var(--space-3) var(--space-4)',
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-md)',
        cursor: 'pointer',
        transition: 'border-color var(--motion-fast)',
        minWidth: '140px',
        textAlign: 'left',
      }}
    >
      <span
        style={{
          fontSize: '14px',
          fontWeight: 600,
          color: 'var(--color-text-primary)',
        }}
      >
        {label}
      </span>
      <span
        style={{
          fontSize: '12px',
          color: 'var(--color-text-muted)',
          marginTop: '2px',
        }}
      >
        {description}
      </span>
    </button>
  )
}
