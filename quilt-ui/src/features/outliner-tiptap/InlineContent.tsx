import { lazy, Suspense, useMemo, useRef, useCallback, useState, useEffect, type ReactNode } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { Calendar, Clock, User, FileText, Tag, Hash } from 'lucide-react'
import type { Block, Page } from '@shared/types/api'
import type { Tab } from '@shared/contexts/TabsContext'
import { useWasm, ensureWasmLoaded } from '@core/wasm-bridge/WasmProvider'
import { api } from '@core/api-client'

// HoverPreview is a heavy popover (it fetches and renders a sub-page).
// We only need its bundle on the rare hover event, not for every block.
const HoverPreview = lazy(() =>
  import('@shared/components/HoverPreview').then(m => ({ default: m.HoverPreview })),
)

// ──── Types ──────────────────────────────────────────────────────────

interface LinkValue {
  text: string
  url: string
}

interface HeaderValue {
  level: number
  text: string
}

interface PropertyValue {
  key: string
  value: string
}

interface SegmentBase {
  type: string
  value: any
}

interface HoverInfo {
  type: 'page' | 'block'
  target: string
  anchorRect: DOMRect
}

interface InlineContentProps {
  content: string
  isEditing?: boolean
  blocks?: Block[]
  pageMap?: Map<string, Page>
  openTab?: (tab: Omit<Tab, 'id'>) => string
}

// The Rust parser serializes `Segment` as an externally-tagged enum by default,
// e.g. `{ "Bold": { "content": "x", ... } }`, while the early TS tests in
// this repo mocked a simplified `{ type: 'bold', value: 'x' }` shape.
//
// Production must support BOTH:
//   - real Rust shape (externally-tagged enum)
//   - simplified test shape (for existing test fixtures)
//
// This normalizer converts either form into the canonical TS shape expected by
// `renderSegment`. If the segment cannot be understood, it returns `null` so
// the caller can fall back to raw text.
function normalizeSegment(seg: any): SegmentBase | null {
  if (!seg || typeof seg !== 'object') return null

  // Already normalized (used by unit tests / hand-built fixtures)
  if (typeof seg.type === 'string' && 'value' in seg) return seg as SegmentBase

  const keys = Object.keys(seg)
  if (keys.length !== 1) return null

  const tag = keys[0]
  const payload = seg[tag]
  if (!payload || typeof payload !== 'object') return null

  switch (tag) {
    case 'Text':
      return { type: 'text', value: payload.content ?? '' }
    case 'PageRef': {
      // G1: `[[Page|alias]]` carries an optional `alias` field from the
      // Rust parser. When present, emit the object shape `{ pageName,
      // alias }`; otherwise keep the legacy plain-string shape for
      // backward compat with existing test fixtures.
      const pageName: string = payload.page_name ?? ''
      const alias: string | null = payload.alias ?? null
      return {
        type: 'pageRef',
        value: alias ? { pageName, alias } : pageName,
      }
    }
    case 'BlockRef':
      return { type: 'blockRef', value: payload.block_uuid ?? '' }
    case 'Tag':
      return { type: 'tag', value: payload.name ?? '' }
    case 'Property':
      return {
        type: 'property',
        value: {
          key: payload.key ?? '',
          value: payload.value ?? '',
        },
      }
    case 'Bold':
      return { type: 'bold', value: payload.content ?? '' }
    case 'Italic':
      return { type: 'italic', value: payload.content ?? '' }
    case 'Code':
      return { type: 'code', value: payload.content ?? '' }
    case 'Link':
      return {
        type: 'link',
        value: {
          text: payload.text ?? '',
          url: payload.url ?? '',
        },
      }
    case 'BoldItalic':
      return { type: 'boldItalic', value: payload.content ?? '' }
    case 'Strikethrough':
      return { type: 'strikethrough', value: payload.content ?? '' }
    case 'Highlight':
      return { type: 'highlight', value: payload.content ?? '' }
    case 'Header':
      return {
        type: 'header',
        value: {
          level: payload.level ?? 1,
          text: payload.content ?? '',
        },
      }
    default:
      return null
  }
}

// ──── Segment renderer ──────────────────────────────────────────────

function renderTextWithNewlines(text: string, key: number): ReactNode {
  if (!text.includes('\n')) {
    return <span key={key}>{text}</span>
  }
  const parts: ReactNode[] = []
  text.split('\n').forEach((line, i) => {
    if (i > 0) parts.push(<br key={`${key}-br-${i}`} />)
    parts.push(line)
  })
  return <span key={key}>{parts}</span>
}

function renderSegment(
  seg: SegmentBase,
  key: number,
  isEditing?: boolean,
  blocks?: Block[],
  pageMap?: Map<string, Page>,
  onHover?: (info: HoverInfo | null) => void,
  onPageRefClick?: (target: string, e: React.MouseEvent) => void,
  onBlockRefClick?: (pageName: string, blockId: string, e: React.MouseEvent) => void,
  onTagClick?: (tagName: string, e: React.MouseEvent) => void,
): ReactNode {
  switch (seg.type) {
    case 'text':
      return renderTextWithNewlines(seg.value, key)

    case 'bold':
      return <strong key={key} style={{ fontWeight: 600 }}>{seg.value}</strong>

    case 'italic':
      return <em key={key}>{seg.value}</em>

    case 'boldItalic':
      return (
        <strong key={key}>
          <em>{seg.value}</em>
        </strong>
      )

    case 'code':
      return (
        <code
          key={key}
          style={{
            background: 'var(--color-surface-subtle)',
            padding: '1px 5px',
            borderRadius: 'var(--radius-sm)',
            fontSize: '0.875em',
            fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
            color: 'var(--color-danger)',
          }}
        >
          {seg.value}
        </code>
      )

    case 'pageRef': {
      // G1: support both legacy `{ value: 'PageName' }` and the new
      // `{ value: { pageName, alias } }` shape. The href and the page
      // lookup always use `pageName`; only the displayed text may
      // differ when an alias is present.
      const rawValue = seg.value
      const { pageName, alias } =
        typeof rawValue === 'string'
          ? { pageName: rawValue as string, alias: null as string | null }
          : {
              pageName: (rawValue as { pageName: string }).pageName,
              alias: (rawValue as { alias: string | null }).alias ?? null,
            }
      const displayText = alias ?? pageName
      const pageExists = pageMap?.has(pageName) ?? false
      return (
        <a
          key={key}
          href={`/page/${encodeURIComponent(pageName)}`}
          onClick={(e) => {
            // CRITICAL: stop propagation so the click doesn't bubble up
            // to BlockRow's onClick and put the block in edit mode
            // (user wants to navigate to the linked page, not edit it).
            e.stopPropagation()
            onPageRefClick?.(pageName, e)
          }}
          style={{
            color: pageExists ? 'var(--color-link)' : 'var(--color-text-disabled)',
            cursor: 'pointer',
            textDecoration: 'none',
            opacity: pageExists ? 1 : 0.6,
            fontWeight: 500,
            borderRadius: '2px',
            padding: pageExists ? '1px 10px' : '0',
            margin: pageExists ? '0 3px' : '0',
            background: pageExists ? 'rgba(37, 99, 235, 0.06)' : 'transparent',
            transition: 'background var(--motion-fast) var(--ease-standard), color var(--motion-fast) var(--ease-standard)',
          }}
          onMouseEnter={(e) => {
            const rect = e.currentTarget.getBoundingClientRect()
            onHover?.({ type: 'page', target: pageName, anchorRect: rect })
            if (pageExists) e.currentTarget.style.background = 'rgba(37, 99, 235, 0.10)'
          }}
          onMouseLeave={(e) => {
            onHover?.(null)
            if (pageExists) e.currentTarget.style.background = 'rgba(37, 99, 235, 0.06)'
          }}
        >
          {displayText}
        </a>
      )
    }

    case 'blockRef': {
      const blockId: string = seg.value
      const refBlock = blocks?.find(b => b.id === blockId)

      if (!refBlock) {
        // The block reference points to a block that doesn't exist in
        // the current pageMap. Render a non-interactive placeholder and
        // explicitly stop click propagation so the user doesn't get
        // bounced into edit mode by a click on dead text. Without this
        // guard the click bubbles to BlockRow, which interprets it as
        // "start editing".
        return (
          <span
            key={key}
            onClick={(e) => e.stopPropagation()}
            style={{
              color: 'var(--color-text-disabled)',
              fontStyle: 'italic',
              fontSize: '0.9em',
              cursor: 'not-allowed',
            }}
          >
            (missing block)
          </span>
        )
      }

      const content = refBlock.content
      const preview = content.length > 80 ? content.substring(0, 80) + '…' : content
      const sourcePageName = refBlock.pageName || refBlock.pageId

      return (
        <span
          key={key}
          onClick={(e) => {
            e.stopPropagation()
            onBlockRefClick?.(sourcePageName, blockId, e)
          }}
          onMouseEnter={(e) => {
            const rect = e.currentTarget.getBoundingClientRect()
            onHover?.({ type: 'block', target: blockId, anchorRect: rect })
          }}
          onMouseLeave={() => onHover?.(null)}
          style={{
            display: 'inline-block',
            padding: '3px 8px',
            background: 'var(--color-surface-subtle)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-md)',
            color: 'var(--color-text-secondary)',
            fontSize: '12px',
            cursor: 'pointer',
            maxWidth: '300px',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            verticalAlign: 'middle',
            marginRight: '4px',
            boxShadow: 'var(--shadow-sm)',
          }}
          title={`Block ref: ${blockId}`}
        >
          <Hash size={11} style={{ display: 'inline', marginRight: '3px', verticalAlign: 'middle' }} />
          {preview}
        </span>
      )
    }

    case 'tag': {
      const tagName: string = seg.value
      return (
        <span
          key={key}
          onClick={(e) => {
            e.stopPropagation()
            onTagClick?.(tagName, e)
          }}
          onMouseEnter={(e) => {
            const rect = e.currentTarget.getBoundingClientRect()
            onHover?.({ type: 'page', target: tagName, anchorRect: rect })
          }}
          onMouseLeave={() => onHover?.(null)}
          style={{
            background: 'var(--color-primary-container)',
            color: 'var(--color-primary)',
            fontSize: '12px',
            fontWeight: 600,
            padding: '1px 6px',
            borderRadius: 'var(--radius-pill)',
            marginLeft: '2px',
            marginRight: '2px',
            cursor: 'pointer',
            transition: 'opacity var(--motion-fast) var(--ease-standard)',
          }}
        >
          #{tagName}
        </span>
      )
    }

    case 'link': {
      const { text, url } = seg.value as LinkValue
      return (
        <a
          key={key}
          href={url}
          target="_blank"
          rel="noopener noreferrer"
          style={{ color: 'var(--color-link)', textDecoration: 'underline' }}
        >
          {text}
        </a>
      )
    }

    case 'strikethrough':
      return <s key={key} style={{ opacity: 0.6 }}>{seg.value}</s>

    case 'highlight':
      return (
        <mark
          key={key}
          style={{
            background: 'var(--color-warning)',
            color: '#000',
            padding: '0 2px',
            borderRadius: '2px',
            opacity: 0.8,
          }}
        >
          {seg.value}
        </mark>
      )

    case 'header': {
      const { level, text } = seg.value as HeaderValue
      const sizes: Record<number, string> = { 1: '28px', 2: '24px', 3: '20px', 4: '18px', 5: '16px', 6: '14px' }
      return (
        <span key={key} style={{ fontSize: sizes[level] || '16px', fontWeight: 700 }}>
          {text}
        </span>
      )
    }

    case 'property': {
      const { key: propKey, value: propValue } = seg.value as PropertyValue

      // Edit mode: render as plain monospace text
      if (isEditing) {
        return (
          <span key={key} style={{ fontSize: '12px', color: 'var(--color-text-muted)', fontFamily: 'monospace' }}>
            {propKey}:: {propValue}
          </span>
        )
      }

      // ── Status ──────────────────────────────────────────────────────
      if (propKey === 'status') {
        const colors: Record<string, { bg: string; fg: string }> = {
          todo: { bg: 'var(--color-info-subtle)', fg: 'var(--color-info)' },
          doing: { bg: 'var(--color-warning-subtle)', fg: 'var(--color-warning)' },
          done: { bg: 'var(--color-success-subtle)', fg: 'var(--color-success)' },
          now: { bg: 'var(--color-accent-subtle)', fg: 'var(--color-accent)' },
          later: { bg: 'var(--color-surface-subtle)', fg: 'var(--color-text-muted)' },
          cancelled: { bg: 'var(--color-danger-subtle)', fg: 'var(--color-danger)' },
        }
        const c = colors[propValue.toLowerCase()] || { bg: 'var(--color-surface-subtle)', fg: 'var(--color-text-secondary)' }
        return (
          <span key={key} style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            padding: '1px 8px',
            borderRadius: 'var(--radius-pill)',
            background: c.bg,
            color: c.fg,
            fontSize: '11px',
            fontWeight: 600,
            marginRight: '4px',
            verticalAlign: 'middle',
          }}>
            {propValue.toUpperCase()}
          </span>
        )
      }

      // ── Priority ────────────────────────────────────────────────────
      if (propKey === 'priority') {
        const colors: Record<string, string> = { a: 'var(--color-danger)', b: 'var(--color-warning)', c: 'var(--color-info)' }
        const c = colors[propValue.toLowerCase()] || 'var(--color-text-secondary)'
        return (
          <span key={key} style={{
            display: 'inline-flex',
            alignItems: 'center',
            padding: '1px 6px',
            borderRadius: 'var(--radius-sm)',
            background: `${c}20`,
            color: c,
            fontSize: '11px',
            fontWeight: 700,
            marginRight: '4px',
            verticalAlign: 'middle',
            border: `1px solid ${c}40`,
          }}>
            P{propValue.toUpperCase()}
          </span>
        )
      }

      // ── Tags ────────────────────────────────────────────────────────
      if (propKey === 'tags') {
        const tags = propValue.split(',').map(t => t.trim()).filter(Boolean)
        return (
          <span key={key} style={{ display: 'inline', marginRight: '4px' }}>
            {tags.map((tag, i) => (
              <span key={i} style={{
                display: 'inline-flex',
                alignItems: 'center',
                padding: '1px 6px',
                borderRadius: 'var(--radius-pill)',
                background: 'var(--color-info-subtle)',
                color: 'var(--color-info)',
                fontSize: '11px',
                fontWeight: 500,
                marginRight: '4px',
                gap: '3px',
              }}>
                <Tag size={10} />
                {tag}
              </span>
            ))}
          </span>
        )
      }

      // ── Deadline ────────────────────────────────────────────────────
      if (propKey === 'deadline') {
        return (
          <span key={key} style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            padding: '1px 8px',
            borderRadius: 'var(--radius-pill)',
            background: 'var(--color-surface-subtle)',
            color: 'var(--color-text-secondary)',
            fontSize: '11px',
            fontWeight: 500,
            marginRight: '4px',
            verticalAlign: 'middle',
          }}>
            <Calendar size={12} />
            {propValue}
          </span>
        )
      }

      // ── Scheduled ───────────────────────────────────────────────────
      if (propKey === 'scheduled') {
        return (
          <span key={key} style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            padding: '1px 8px',
            borderRadius: 'var(--radius-pill)',
            background: 'var(--color-surface-subtle)',
            color: 'var(--color-text-secondary)',
            fontSize: '11px',
            fontWeight: 500,
            marginRight: '4px',
            verticalAlign: 'middle',
          }}>
            <Clock size={12} />
            {propValue}
          </span>
        )
      }

      // ── Created_by ──────────────────────────────────────────────────
      if (propKey === 'created_by') {
        return (
          <span key={key} style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            padding: '1px 8px',
            borderRadius: 'var(--radius-pill)',
            background: 'var(--color-surface-subtle)',
            color: 'var(--color-text-secondary)',
            fontSize: '11px',
            fontWeight: 500,
            marginRight: '4px',
            verticalAlign: 'middle',
          }}>
            <User size={12} />
            {propValue}
          </span>
        )
      }

      // ── Template ────────────────────────────────────────────────────
      if (propKey === 'template') {
        return (
          <span key={key} style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            padding: '1px 8px',
            borderRadius: 'var(--radius-pill)',
            background: 'var(--color-surface-subtle)',
            color: 'var(--color-text-secondary)',
            fontSize: '11px',
            fontWeight: 500,
            marginRight: '4px',
            verticalAlign: 'middle',
          }}>
            <FileText size={12} />
            {propValue}
          </span>
        )
      }

      // ── Generic property (fallback) ─────────────────────────────────
      return (
        <span key={key} style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
          padding: '1px 8px',
          borderRadius: 'var(--radius-pill)',
          background: 'var(--color-surface-subtle)',
          color: 'var(--color-text-secondary)',
          fontSize: '11px',
          fontWeight: 500,
          marginRight: '4px',
          verticalAlign: 'middle',
          border: '1px solid var(--color-border)',
        }}>
          <span style={{ color: 'var(--color-text-muted)', fontSize: '10px' }}>{propKey}</span>
          <span>{propValue}</span>
        </span>
      )
    }

    default:
      return <span key={key}>{typeof seg.value === 'string' ? seg.value : JSON.stringify(seg.value)}</span>
  }
}

// ──── Component ─────────────────────────────────────────────────────

export function InlineContent({ content, isEditing, blocks, pageMap, openTab }: InlineContentProps) {
  const { loaded, wasmParseInline } = useWasm()
  const [inlineWasmReady, setInlineWasmReady] = useState(loaded)
  const navigate = useNavigate()

  // ── Hover state & timers ────────────────────────────────────────
  const [hoveredRef, setHoveredRef] = useState<HoverInfo | null>(null)
  const showTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const hideTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  const handleHover = useCallback((info: HoverInfo | null) => {
    if (info) {
      clearTimeout(hideTimerRef.current)
      clearTimeout(showTimerRef.current)
      showTimerRef.current = setTimeout(() => {
        setHoveredRef(info)
      }, 300)
    } else {
      clearTimeout(showTimerRef.current)
      hideTimerRef.current = setTimeout(() => {
        setHoveredRef(null)
      }, 200)
    }
  }, [])

  const handlePopoverEnter = useCallback(() => {
    clearTimeout(hideTimerRef.current)
  }, [])

  const handlePopoverLeave = useCallback(() => {
    hideTimerRef.current = setTimeout(() => {
      setHoveredRef(null)
    }, 200)
  }, [])

  const handlePageRefClick = useCallback(async (target: string, e: React.MouseEvent) => {
    e.preventDefault()
    if (e.shiftKey) {
      // TODO: Open in sidebar (sidebar not yet implemented in Quilt).
      // Logseq opens the linked page/block in a sidebar panel here.
      return
    }

    // Logseq behavior: if the page doesn't exist, create it on the fly
    // when the user clicks the link. The client-side `pageMap` is the
    // source of truth for "does it exist"; if it's stale (e.g. the
    // page was created in another tab), the server's UNIQUE constraint
    // will reject the duplicate insert and we just navigate anyway.
    const pageExists = pageMap?.has(target) ?? false
    if (!pageExists) {
      try {
        await api.createPage({ name: target })
      } catch {
        // Concurrent creation or network blip — the navigation below
        // will still work because the page does exist on the server.
      }
    }

    if (e.metaKey || e.ctrlKey) {
      if (openTab) {
        openTab({ name: target, type: 'page', title: target, params: {} })
      }
      return
    }
    // Normal navigation — use TanStack Router (path-based). The
    // earlier `window.location.hash` assignment was a bug: TanStack
    // uses `createBrowserHistory` and never reads the URL hash, so
    // the link silently failed to navigate.
    navigate({ to: '/page/$name', params: { name: target } })
  }, [navigate, openTab, pageMap])

  // Block-ref click. Logseq opens the block in the sidebar (not the
  // main content area) — see handler/editor.cljs open-block-in-sidebar!.
  // Quilt has no sidebar yet, so we navigate to the block's parent
  // page. TODO: once a sidebar exists, add `e.shiftKey` to open there.
  const handleBlockRefClick = useCallback((pageName: string, _blockId: string, e: React.MouseEvent) => {
    e.stopPropagation()
    if (e.metaKey || e.ctrlKey) {
      if (openTab) {
        openTab({ name: pageName, type: 'page', title: pageName, params: {} })
      }
      return
    }
    navigate({ to: '/page/$name', params: { name: pageName } })
  }, [navigate, openTab])

  // #tag click. Logseq treats tags as pages (#tag → navigate to that
  // page; creates the page if it doesn't exist).
  const handleTagClick = useCallback(async (tagName: string, e: React.MouseEvent) => {
    e.stopPropagation()
    e.preventDefault()
    if (e.shiftKey) {
      // TODO: Open in sidebar
      return
    }
    const pageExists = pageMap?.has(tagName) ?? false
    if (!pageExists) {
      try {
        await api.createPage({ name: tagName })
      } catch {
        // Concurrent create — page may already exist on the server.
      }
    }
    if (e.metaKey || e.ctrlKey) {
      if (openTab) {
        openTab({ name: tagName, type: 'page', title: tagName, params: {} })
      }
      return
    }
    navigate({ to: '/page/$name', params: { name: tagName } })
  }, [navigate, openTab, pageMap])

  // Cleanup timers on unmount
  useEffect(() => {
    return () => {
      clearTimeout(showTimerRef.current)
      clearTimeout(hideTimerRef.current)
    }
  }, [])

  // Trigger lazy WASM load on first use. The promise is cached, so
  // calling this from many components does not re-download the engine.
  useEffect(() => {
    let cancelled = false
    if (loaded) {
      setInlineWasmReady(true)
      return
    }
    if (!content) return
    void ensureWasmLoaded()
      .then(() => {
        if (!cancelled) setInlineWasmReady(true)
      })
      .catch(() => {
        if (!cancelled) setInlineWasmReady(false)
      })
    return () => {
      cancelled = true
    }
  }, [loaded, content])

  const segments = useMemo(() => {
    if (!inlineWasmReady || !content) return null
    try {
      const result = wasmParseInline(content)
      const rawSegments = result?.segments || null
      if (!Array.isArray(rawSegments)) return null
      const normalized = rawSegments
        .map(normalizeSegment)
        .filter((s): s is SegmentBase => s !== null)
      return normalized.length > 0 ? normalized : null
    } catch {
      return null
    }
  }, [content, inlineWasmReady, wasmParseInline])

  // Fallback: if WASM not ready or parse fails, render raw content with newlines
  if (!segments || segments.length === 0) {
    if (!content.includes('\n')) {
      return <>{content}</>
    }
    const parts: ReactNode[] = []
    content.split('\n').forEach((line, i) => {
      if (i > 0) parts.push(<br key={`fb-br-${i}`} />)
      parts.push(line)
    })
    return <>{parts}</>
  }

  return (
    <>
      {segments.map((seg: SegmentBase, i: number) =>
        renderSegment(seg, i, isEditing, blocks, pageMap, handleHover, handlePageRefClick, handleBlockRefClick, handleTagClick)
      )}
      {hoveredRef && (
        <Suspense fallback={null}>
          <HoverPreview
            type={hoveredRef.type}
            target={hoveredRef.target}
            anchorRect={hoveredRef.anchorRect}
            onClose={() => setHoveredRef(null)}
            onMouseEnter={handlePopoverEnter}
            onMouseLeave={handlePopoverLeave}
          />
        </Suspense>
      )}
    </>
  )
}
