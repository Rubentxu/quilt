import { useState, useEffect, useRef, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { api } from '@core/api-client'
import { ErrorBoundary } from '@shared/components/ErrorBoundary'
import { useTabs } from '@shared/contexts/TabsContext'
import type { Backlink } from '@shared/types/api'
import toast from 'react-hot-toast'
import { ZoomIn, ZoomOut, Maximize2 } from 'lucide-react'

// ──── Types ────────────────────────────────────────────────

interface GraphNode {
  id: string
  label: string
  x: number
  y: number
  vx: number
  vy: number
  radius: number
  isJournal: boolean
}

interface GraphEdge {
  source: string
  target: string
}

interface PageStub {
  name: string
  title: string | null
  journal: boolean
}

// ──── Force Simulation ─────────────────────────────────────

const REPULSION = 2000
const ATTRACTION = 0.005
const DAMPING = 0.9
const CENTER_GRAVITY = 0.01

function simulate(nodes: GraphNode[], edges: GraphEdge[], width: number, height: number) {
  const centerX = width / 2
  const centerY = height / 2

  // Repulsion between all nodes
  for (let i = 0; i < nodes.length; i++) {
    for (let j = i + 1; j < nodes.length; j++) {
      const dx = nodes[j].x - nodes[i].x
      const dy = nodes[j].y - nodes[i].y
      const dist = Math.sqrt(dx * dx + dy * dy) || 1
      const force = REPULSION / (dist * dist)
      const fx = (dx / dist) * force
      const fy = (dy / dist) * force
      nodes[i].vx -= fx
      nodes[i].vy -= fy
      nodes[j].vx += fx
      nodes[j].vy += fy
    }
  }

  // Attraction along edges
  for (const edge of edges) {
    const source = nodes.find(n => n.id === edge.source)
    const target = nodes.find(n => n.id === edge.target)
    if (!source || !target) continue
    const dx = target.x - source.x
    const dy = target.y - source.y
    const dist = Math.sqrt(dx * dx + dy * dy) || 1
    const force = dist * ATTRACTION
    const fx = (dx / dist) * force
    const fy = (dy / dist) * force
    source.vx += fx
    source.vy += fy
    target.vx -= fx
    target.vy -= fy
  }

  // Center gravity + damping + update positions
  for (const node of nodes) {
    node.vx += (centerX - node.x) * CENTER_GRAVITY
    node.vy += (centerY - node.y) * CENTER_GRAVITY
    node.vx *= DAMPING
    node.vy *= DAMPING
    node.x += node.vx
    node.y += node.vy
    // Keep within bounds
    node.x = Math.max(node.radius, Math.min(width - node.radius, node.x))
    node.y = Math.max(node.radius, Math.min(height - node.radius, node.y))
  }
}

// ──── Component ────────────────────────────────────────────

export function GraphViewPage() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const nodesRef = useRef<GraphNode[]>([])
  const edgesRef = useRef<GraphEdge[]>([])
  const animRef = useRef<number>(0)
  const [zoom, setZoom] = useState(1)
  const [pan, setPan] = useState({ x: 0, y: 0 })
  const [dragging, setDragging] = useState<string | null>(null)
  const [hoveredNode, setHoveredNode] = useState<string | null>(null)
  const [dimensions, setDimensions] = useState({ width: 800, height: 600 })
  const navigate = useNavigate()
  const { openTab } = useTabs()

  // Auto-open tab for graph view
  useEffect(() => {
    openTab({ name: 'graph', type: 'graph', title: 'Knowledge Graph', params: {} })
  }, [openTab])

  // Load data
  useEffect(() => {
    let cancelled = false

    async function loadData() {
      try {
        const pages: PageStub[] = await api.listPages()
        if (cancelled) return

        const width = dimensions.width
        const height = dimensions.height

        // Create nodes
        const pageNames = new Set(pages.map(p => p.name))
        const nodes: GraphNode[] = pages.map((page, i) => ({
          id: page.name,
          label: page.title || page.name,
          x: width / 2 + (Math.random() - 0.5) * 300,
          y: height / 2 + (Math.random() - 0.5) * 300,
          vx: 0,
          vy: 0,
          radius: page.journal ? 6 : 8,
          isJournal: page.journal,
        }))

        // Create edges from backlinks
        const edges: GraphEdge[] = []
        const edgeSet = new Set<string>()

        // Only fetch backlinks for first 50 pages to avoid hammering
        const pagesToFetch = pages.slice(0, 50)
        const results = await Promise.allSettled(
          pagesToFetch.map(page =>
            api.getPageBacklinks(page.name)
              .then((backlinks: Backlink[]) => ({ pageName: page.name, backlinks }))
          )
        )

        for (const result of results) {
          if (result.status !== 'fulfilled') continue
          const { pageName, backlinks } = result.value
          for (const bl of backlinks) {
            // Only create edges to pages that exist in our node set
            if (!pageNames.has(bl.sourcePageName)) continue
            const edgeKey = `${bl.sourcePageName}->${pageName}`
            if (!edgeSet.has(edgeKey)) {
              edgeSet.add(edgeKey)
              edges.push({ source: bl.sourcePageName, target: pageName })
            }
          }
        }

        if (!cancelled) {
          nodesRef.current = nodes
          edgesRef.current = edges
        }
      } catch {
        if (!cancelled) toast.error('Failed to load graph data')
      }
    }

    loadData()
    return () => { cancelled = true }
  }, [dimensions.width, dimensions.height])

  // Resize observer
  useEffect(() => {
    if (!containerRef.current) return
    const observer = new ResizeObserver(entries => {
      for (const entry of entries) {
        setDimensions({
          width: entry.contentRect.width,
          height: entry.contentRect.height,
        })
      }
    })
    observer.observe(containerRef.current)
    return () => observer.disconnect()
  }, [])

  // Animation loop
  useEffect(() => {
    const dpr = window.devicePixelRatio || 1

    function draw() {
      const canvas = canvasRef.current
      if (!canvas) return
      const ctx = canvas.getContext('2d')
      if (!ctx) {
        animRef.current = requestAnimationFrame(draw)
        return
      }

      const nodes = nodesRef.current
      const edges = edgesRef.current
      if (nodes.length === 0) {
        animRef.current = requestAnimationFrame(draw)
        return
      }

      // Run simulation
      simulate(nodes, edges, dimensions.width, dimensions.height)

      // Clear with DPR scaling
      canvas.width = dimensions.width * dpr
      canvas.height = dimensions.height * dpr
      canvas.style.width = `${dimensions.width}px`
      canvas.style.height = `${dimensions.height}px`
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
      ctx.clearRect(0, 0, dimensions.width, dimensions.height)

      ctx.save()
      ctx.translate(pan.x, pan.y)
      ctx.scale(zoom, zoom)

      // Canvas doesn't support CSS custom properties — detect theme
      const isDark = document.documentElement.classList.contains('dark')

      // Draw edges
      ctx.strokeStyle = isDark ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.1)'
      ctx.lineWidth = 1
      for (const edge of edges) {
        const source = nodes.find(n => n.id === edge.source)
        const target = nodes.find(n => n.id === edge.target)
        if (!source || !target) continue
        ctx.beginPath()
        ctx.moveTo(source.x, source.y)
        ctx.lineTo(target.x, target.y)
        ctx.stroke()
      }

      // Draw nodes
      for (const node of nodes) {
        const isHovered = hoveredNode === node.id
        const isConnected = hoveredNode
          ? edges.some(e =>
            (e.source === hoveredNode && e.target === node.id) ||
            (e.target === hoveredNode && e.source === node.id)
          ) || node.id === hoveredNode
          : true

        // Node circle
        ctx.beginPath()
        ctx.arc(node.x, node.y, isHovered ? node.radius + 3 : node.radius, 0, Math.PI * 2)

        if (node.isJournal) {
          ctx.fillStyle = isDark
            ? (isConnected ? 'rgba(59,130,246,0.8)' : 'rgba(59,130,246,0.2)')
            : (isConnected ? 'rgba(37,99,235,0.8)' : 'rgba(37,99,235,0.2)')
        } else {
          ctx.fillStyle = isDark
            ? (isConnected ? 'rgba(168,162,158,0.8)' : 'rgba(168,162,158,0.2)')
            : (isConnected ? 'rgba(87,83,78,0.8)' : 'rgba(87,83,78,0.2)')
        }
        ctx.fill()

        // Label
        if (isConnected && (isHovered || zoom > 0.6)) {
          ctx.font = `${isHovered ? '12px' : '10px'} Inter, sans-serif`
          ctx.fillStyle = isDark
            ? (isHovered ? 'rgba(255,255,255,0.9)' : 'rgba(255,255,255,0.5)')
            : (isHovered ? 'rgba(0,0,0,0.9)' : 'rgba(0,0,0,0.5)')
          ctx.textAlign = 'center'
          ctx.fillText(
            node.label.length > 20 ? node.label.slice(0, 18) + '…' : node.label,
            node.x,
            node.y + node.radius + 14
          )
        }
      }

      ctx.restore()
      animRef.current = requestAnimationFrame(draw)
    }

    animRef.current = requestAnimationFrame(draw)
    return () => cancelAnimationFrame(animRef.current)
  }, [zoom, pan, dimensions, hoveredNode])

  // Mouse interaction
  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current
    if (!canvas) return
    const rect = canvas.getBoundingClientRect()
    const mx = (e.clientX - rect.left - pan.x) / zoom
    const my = (e.clientY - rect.top - pan.y) / zoom

    if (dragging) {
      const node = nodesRef.current.find(n => n.id === dragging)
      if (node) {
        node.x = mx
        node.y = my
        node.vx = 0
        node.vy = 0
      }
      return
    }

    // Hit test for hover
    const hit = nodesRef.current.find(n => {
      const dx = n.x - mx
      const dy = n.y - my
      return Math.sqrt(dx * dx + dy * dy) < n.radius + 5
    })
    setHoveredNode(hit?.id ?? null)
    canvas.style.cursor = hit ? 'pointer' : 'grab'
  }, [zoom, pan, dragging])

  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current
    if (!canvas) return
    const rect = canvas.getBoundingClientRect()
    const mx = (e.clientX - rect.left - pan.x) / zoom
    const my = (e.clientY - rect.top - pan.y) / zoom

    const hit = nodesRef.current.find(n => {
      const dx = n.x - mx
      const dy = n.y - my
      return Math.sqrt(dx * dx + dy * dy) < n.radius + 5
    })

    if (hit) {
      setDragging(hit.id)
      canvas.style.cursor = 'grabbing'
    } else {
      // Start panning — track initial position
      const startPan = { ...pan }
      const startX = e.clientX
      const startY = e.clientY

      const onMouseMove = (ev: globalThis.MouseEvent) => {
        setPan({
          x: startPan.x + (ev.clientX - startX),
          y: startPan.y + (ev.clientY - startY),
        })
      }
      const onMouseUp = () => {
        window.removeEventListener('mousemove', onMouseMove)
        window.removeEventListener('mouseup', onMouseUp)
      }
      window.addEventListener('mousemove', onMouseMove)
      window.addEventListener('mouseup', onMouseUp)
    }
  }, [zoom, pan])

  const handleMouseUp = useCallback(() => {
    setDragging(null)
  }, [])

  const handleDoubleClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current
    if (!canvas) return
    const rect = canvas.getBoundingClientRect()
    const mx = (e.clientX - rect.left - pan.x) / zoom
    const my = (e.clientY - rect.top - pan.y) / zoom

    const hit = nodesRef.current.find(n => {
      const dx = n.x - mx
      const dy = n.y - my
      return Math.sqrt(dx * dx + dy * dy) < n.radius + 5
    })

    if (hit) {
      navigate({ to: '/page/$name', params: { name: hit.id } })
    }
  }, [zoom, pan, navigate])

  // Wheel zoom
  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault()
    const delta = e.deltaY > 0 ? 0.9 : 1.1
    setZoom(z => Math.max(0.2, Math.min(3, z * delta)))
  }, [])

  return (
    <ErrorBoundary>
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Header */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        padding: 'var(--space-4)',
        borderBottom: '1px solid var(--color-border)',
      }}>
        <h2 style={{
          fontSize: '20px',
          fontWeight: 700,
          color: 'var(--color-text-primary)',
        }}>
          Knowledge Graph
        </h2>
        <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
          <button
            onClick={() => setZoom(z => Math.min(3, z * 1.2))}
            title="Zoom in"
            style={{
              background: 'var(--color-surface)',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              padding: 'var(--space-1) var(--space-2)',
              cursor: 'pointer',
              color: 'var(--color-text-secondary)',
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <ZoomIn size={16} />
          </button>
          <button
            onClick={() => setZoom(z => Math.max(0.2, z * 0.8))}
            title="Zoom out"
            style={{
              background: 'var(--color-surface)',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              padding: 'var(--space-1) var(--space-2)',
              cursor: 'pointer',
              color: 'var(--color-text-secondary)',
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <ZoomOut size={16} />
          </button>
          <button
            onClick={() => { setZoom(1); setPan({ x: 0, y: 0 }) }}
            title="Reset view"
            style={{
              background: 'var(--color-surface)',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              padding: 'var(--space-1) var(--space-2)',
              cursor: 'pointer',
              color: 'var(--color-text-secondary)',
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <Maximize2 size={16} />
          </button>
        </div>
      </div>

      {/* Canvas */}
      <div ref={containerRef} style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
        <canvas
          ref={canvasRef}
          onMouseMove={handleMouseMove}
          onMouseDown={handleMouseDown}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseUp}
          onDoubleClick={handleDoubleClick}
          onWheel={handleWheel}
          style={{
            display: 'block',
            width: '100%',
            height: '100%',
          }}
        />

        {/* Info overlay */}
        <div style={{
          position: 'absolute',
          bottom: 'var(--space-4)',
          left: 'var(--space-4)',
          fontSize: '12px',
          color: 'var(--color-text-muted)',
          background: 'var(--color-surface)',
          padding: 'var(--space-2) var(--space-3)',
          borderRadius: 'var(--radius-sm)',
          border: '1px solid var(--color-border)',
        }}>
          {nodesRef.current.length} nodes · {edgesRef.current.length} links · zoom {Math.round(zoom * 100)}%
        </div>
      </div>
    </div>
    </ErrorBoundary>
  )
}
