// ─── CognitiveGraph component tests ───────────────────────────────────────────
//
// CG-2: Cognitive Dashboard / Graph View.
// Tests the 4 states (loading / error / empty / with-data) and actions.

import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { CognitiveGraph } from '../CognitiveGraph'
import { api } from '@core/api-client'
import type { CognitiveGraphDto } from '@shared/types/api'

// ─── Mock data ────────────────────────────────────────────────────────────────

const mockNodeA = {
  id: 'node-a-1',
  blockId: 'block-a-1',
  pageId: 'page-1',
  pageName: 'Test Page',
  contentPreview: 'This is a test block about Rust',
  influenceScore: 0.8,
  isFrontier: true,
  isGap: false,
  clusterId: 'cluster-0',
}

const mockNodeB = {
  id: 'node-b-1',
  blockId: 'block-b-1',
  pageId: 'page-1',
  pageName: 'Test Page',
  contentPreview: 'Another test block',
  influenceScore: 0.3,
  isFrontier: false,
  isGap: true,
  clusterId: null,
}

const mockNodeC = {
  id: 'node-c-1',
  blockId: 'block-c-1',
  pageId: 'page-2',
  pageName: 'Another Page',
  contentPreview: 'Third block in a different page',
  influenceScore: 0.5,
  isFrontier: false,
  isGap: false,
  clusterId: 'cluster-1',
}

const mockCluster0 = {
  id: 'cluster-0',
  blockIds: ['block-a-1'],
  theme: null,
  coherenceScore: 0.9,
}

const mockCluster1 = {
  id: 'cluster-1',
  blockIds: ['block-c-1'],
  theme: null,
  coherenceScore: 0.7,
}

const mockDto: CognitiveGraphDto = {
  nodes: [mockNodeA, mockNodeB, mockNodeC],
  edges: [{ from: 'block-a-1', to: 'block-b-1' }],
  clusters: [mockCluster0, mockCluster1],
  frontierNodes: ['node-a-1'],
  gapNodes: ['node-b-1'],
  generatedAt: new Date().toISOString(),
}

const mockEmptyDto: CognitiveGraphDto = {
  nodes: [],
  edges: [],
  clusters: [],
  frontierNodes: [],
  gapNodes: [],
  generatedAt: new Date().toISOString(),
}

// ─── Mock API ────────────────────────────────────────────────────────────────

vi.mock('@core/api-client', () => ({
  api: {
    getCognitiveGraph: vi.fn(),
  },
}))

describe('CognitiveGraph', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ─── Loading state ─────────────────────────────────────────────────────

  it('shows loading state initially', async () => {
    vi.mocked(api.getCognitiveGraph).mockImplementation(
      () => new Promise(() => {}), // never resolves
    )

    render(<CognitiveGraph />)

    expect(screen.getByTestId('cognitive-graph-loading')).toBeInTheDocument()
    expect(screen.getByText('Loading cognitive graph…')).toBeInTheDocument()
  })

  // ─── Error state ───────────────────────────────────────────────────────

  it('shows error state when API fails', async () => {
    vi.mocked(api.getCognitiveGraph).mockRejectedValue(new Error('Network error'))

    render(<CognitiveGraph />)

    await waitFor(() => {
      expect(screen.getByTestId('cognitive-graph-error')).toBeInTheDocument()
    })
    expect(screen.getByText('Network error')).toBeInTheDocument()
  })

  // ─── Error retry ───────────────────────────────────────────────────────

  it('retry button reloads the graph after error', async () => {
    vi.mocked(api.getCognitiveGraph).mockRejectedValue(new Error('Network error'))

    render(<CognitiveGraph />)

    await waitFor(() => {
      expect(screen.getByTestId('cognitive-graph-error')).toBeInTheDocument()
    })

    vi.mocked(api.getCognitiveGraph).mockResolvedValue(mockDto)

    const user = userEvent.setup()
    await user.click(screen.getByTestId('cognitive-graph-retry'))

    await waitFor(() => {
      expect(screen.getByTestId('cognitive-graph')).toBeInTheDocument()
    })
  })

  // ─── Empty state ───────────────────────────────────────────────────────

  it('shows empty state when graph is cold', async () => {
    vi.mocked(api.getCognitiveGraph).mockResolvedValue(mockEmptyDto)

    render(<CognitiveGraph />)

    await waitFor(() => {
      expect(screen.getByTestId('cognitive-graph-empty')).toBeInTheDocument()
    })
    expect(
      screen.getByText('No graph data yet — create some pages and link blocks to see the graph.'),
    ).toBeInTheDocument()
  })

  // ─── With-data: stats bar ─────────────────────────────────────────────────────

  it('renders stats bar with correct counts', async () => {
    vi.mocked(api.getCognitiveGraph).mockResolvedValue(mockDto)

    render(<CognitiveGraph />)

    await waitFor(() => {
      expect(screen.getByTestId('stat-nodes')).toBeInTheDocument()
    })
    expect(screen.getByTestId('stat-nodes')).toHaveTextContent('3')
    expect(screen.getByTestId('stat-edges')).toHaveTextContent('1')
    expect(screen.getByTestId('stat-clusters')).toHaveTextContent('2')
    expect(screen.getByTestId('stat-frontier')).toHaveTextContent('1')
    expect(screen.getByTestId('stat-gaps')).toHaveTextContent('1')
  })

  // ─── With-data: frontier section ─────────────────────────────────────────────────

  it('renders frontier nodes section', async () => {
    vi.mocked(api.getCognitiveGraph).mockResolvedValue(mockDto)

    render(<CognitiveGraph />)

    await waitFor(() => {
      expect(screen.getByText('Frontier hubs (1)')).toBeInTheDocument()
    })
    expect(screen.getByTestId('badge-frontier')).toBeInTheDocument()
  })

  // ─── With-data: gap section ─────────────────────────────────────────────────────

  it('renders gap nodes section', async () => {
    vi.mocked(api.getCognitiveGraph).mockResolvedValue(mockDto)

    render(<CognitiveGraph />)

    await waitFor(() => {
      expect(screen.getByText('Gap / isolated (1)')).toBeInTheDocument()
    })
    expect(screen.getByTestId('badge-gap')).toBeInTheDocument()
  })

  // ─── With-data: all nodes section ─────────────────────────────────────────────────────

  it('renders all nodes', async () => {
    vi.mocked(api.getCognitiveGraph).mockResolvedValue(mockDto)

    render(<CognitiveGraph />)

    await waitFor(() => {
      expect(screen.getByTestId('cognitive-graph')).toBeInTheDocument()
    })
    expect(screen.getByText('All nodes (3)')).toBeInTheDocument()
  })

  // ─── With-data: node click calls onNavigate ─────────────────────────────────────

  it('calls onNavigate when a node is clicked', async () => {
    vi.mocked(api.getCognitiveGraph).mockResolvedValue(mockDto)

    const onNavigate = vi.fn()
    render(<CognitiveGraph onNavigate={onNavigate} />)

    await waitFor(() => {
      expect(screen.getByTestId('cognitive-graph')).toBeInTheDocument()
    })

    const user = userEvent.setup()
    // Find the first node card button (role="button")
    const nodeCard = screen.getByTestId('node-node-a-1')
    await user.click(nodeCard)

    expect(onNavigate).toHaveBeenCalledWith('block-a-1', 'Test Page')
  })

  // ─── Refresh button ─────────────────────────────────────────────────────

  it('refresh button reloads data', async () => {
    vi.mocked(api.getCognitiveGraph).mockResolvedValue(mockDto)

    render(<CognitiveGraph />)

    await waitFor(() => {
      expect(screen.getByTestId('cognitive-graph')).toBeInTheDocument()
    })

    vi.mocked(api.getCognitiveGraph).mockResolvedValue({
      ...mockDto,
      nodes: [{ ...mockNodeA, contentPreview: 'Updated content' }],
    })

    const user = userEvent.setup()
    await user.click(screen.getByTestId('cognitive-graph-refresh'))

    await waitFor(() => {
      expect(vi.mocked(api.getCognitiveGraph)).toHaveBeenCalledTimes(2)
    })
  })

  // ─── Cluster toggle ─────────────────────────────────────────────────────

  it('cluster section is expandable', async () => {
    vi.mocked(api.getCognitiveGraph).mockResolvedValue(mockDto)

    render(<CognitiveGraph />)

    await waitFor(() => {
      expect(screen.getByTestId('cognitive-graph')).toBeInTheDocument()
    })

    // Initially clusters section should show toggle button
    expect(screen.getByTestId('cluster-toggle')).toBeInTheDocument()
    expect(screen.getByText('Clusters (2)')).toBeInTheDocument()

    const user = userEvent.setup()
    await user.click(screen.getByTestId('cluster-toggle'))

    // After clicking, should show cluster content
    await waitFor(() => {
      expect(screen.getByTestId('cluster-cluster-0')).toBeInTheDocument()
    })
  })
})
