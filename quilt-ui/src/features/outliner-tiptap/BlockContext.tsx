import { createContext, useContext, type ReactNode } from 'react'
import type { Block, Page } from '@shared/types/api'
import type { Tab } from '@shared/contexts/TabsContext'

export interface BlockContextValue {
  block: Block
  allBlocks: Block[]
  pageName: string
  pageMap: Map<string, Page>
  isEditing: boolean
  setEditing: (editing: boolean) => void
  onSave: (content: string) => Promise<void> | void
  onUpdate: (block: Block) => void
  onDeleteBlock: (blockId: string) => void
  onCreateBlock: (afterBlockId: string, content: string, parentId: string | null) => void
  onFocusBlock: (blockId: string, cursorPos: 'start' | 'end') => void
  openTab?: (tab: Omit<Tab, 'id'>) => string
}

const BlockContext = createContext<BlockContextValue | null>(null)

export function BlockContextProvider({ value, children }: { value: BlockContextValue; children: ReactNode }) {
  return <BlockContext.Provider value={value}>{children}</BlockContext.Provider>
}

export function useOptionalBlockContext(): BlockContextValue | null {
  return useContext(BlockContext)
}
