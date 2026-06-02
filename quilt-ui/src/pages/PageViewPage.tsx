import { useEffect } from 'react'
import { useParams } from '@tanstack/react-router'
import { AlertCircle } from 'lucide-react'
import { PageView } from '@features/outliner-tiptap/PageView'
import { ErrorBoundary } from '@shared/components/ErrorBoundary'
import { useTabs } from '@shared/contexts/TabsContext'

export function PageViewPage() {
  const { name } = useParams({ from: '/page/$name' })
  const { openTab } = useTabs()

  // Auto-open tab for this page
  useEffect(() => {
    const decoded = decodeURIComponent(name)
    openTab({
      name: decoded,
      type: 'page',
      title: decoded,
      params: { name: decoded },
    })
  }, [name, openTab])
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
      <PageView pageName={decodeURIComponent(name)} />
    </ErrorBoundary>
  )
}
