import { useEffect } from 'react'
import { useParams } from '@tanstack/react-router'
import { AlertCircle } from 'lucide-react'
import { PageView } from '@features/outliner-tiptap/PageView'
import { ErrorBoundary } from '@shared/components/ErrorBoundary'
import { useTabs } from '@shared/contexts/TabsContext'
import { useUrlParam } from '@shared/hooks/useUrlParam'
import { usePanelVisibility } from '@features/dashboard'
import { useFocusMode } from '@features/focus-mode/FocusModeContext'
import { FocusModeLayout } from '@features/focus-mode/FocusModeLayout'

export function PageViewPage() {
  const { name } = useParams({ from: '/page/$name' })
  const { openTab } = useTabs()
  const [zoomBlockId, setZoomBlockId] = useUrlParam('zoom')
  const { setVisiblePanels } = usePanelVisibility()
  const { isActive: isFocusMode } = useFocusMode()
  const decodedName = decodeURIComponent(name)

  // Auto-open tab for this page
  useEffect(() => {
    openTab({
      name: decodedName,
      type: 'page',
      title: decodedName,
      // params intentionally empty — the tab id is derived from `name` only.
      // Putting `name` in params would create a different identity than other
      // openTab call sites (e.g. AppShell's Ctrl+T, InlineContent's link
      // click) and break dedup.
      params: {},
    })
  }, [name, openTab, decodedName])

  // Hide sidebar and cognitive panels when focus mode is active
  useEffect(() => {
    if (isFocusMode) {
      // Hide sidebar, backlinks, and all cognitive panels in focus mode
      setVisiblePanels(new Set(['backlinks']))
    }
  }, [isFocusMode, setVisiblePanels])

  return (
    <ErrorBoundary
      fallback={
        <div style={{ padding: 'var(--space-4)', textAlign: 'center', color: 'var(--color-text-muted)' }}>
          <AlertCircle size={20} />
          <p>Failed to load page</p>
          <button onClick={() => window.location.reload()}>Reload</button>
        </div>
      }
    >
      <FocusModeLayout>
        <PageView
          pageName={decodedName}
          zoomBlockId={zoomBlockId}
          onZoomOut={() => setZoomBlockId(null)}
        />
      </FocusModeLayout>
    </ErrorBoundary>
  )
}
