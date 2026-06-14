import type { Dispatch, SetStateAction } from 'react'
import { useBlockHistory } from '@shared/hooks/useBlockHistory'
import { useUndoManager } from '@shared/hooks/useUndoManager'
import type { Block } from '@shared/types/api'
import { api } from '@core/api-client'

interface UsePageUndoManagerArgs {
  pageName: string
  blocks: Block[]
  setBlocks: Dispatch<SetStateAction<Block[]>>
  wasmLoaded: boolean
}

export function usePageUndoManager({ pageName, blocks, setBlocks, wasmLoaded }: UsePageUndoManagerArgs) {
  const history = useBlockHistory({
    pageName,
    blocks,
    onBlocksChanged: setBlocks,
    enabled: wasmLoaded && !!pageName,
    onAfterHistoryChange: (changed) => {
      if (changed.length === 0) return
      const results = Promise.all(
        changed.map(block =>
          api.updateBlock(block.id, { content: block.content }, block.pageName ?? pageName)
            .then(() => true)
            .catch((error) => {
              console.error('useBlockHistory: persist undo/redo failed', error)
              return false
            }),
        ),
      )

      results.then((oks) => {
        if (!oks.some(ok => !ok)) return
        api.getPageBlocks(pageName)
          .then(setBlocks)
          .catch(() => {})
      })
    },
  })

  const sessionUndo = useUndoManager(50)

  return {
    ...history,
    pushUndo: sessionUndo.push,
    undoLast: sessionUndo.undo,
    sessionCanUndo: sessionUndo.canUndo,
  }
}
