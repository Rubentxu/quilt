import { createContext, useContext, useState, type ReactNode } from 'react'

interface ConnectionContextValue {
  sseConnected: boolean
  setSseConnected: (v: boolean) => void
}

const ConnectionContext = createContext<ConnectionContextValue>({
  sseConnected: false,
  setSseConnected: () => {},
})

export function ConnectionProvider({ children }: { children: ReactNode }) {
  const [sseConnected, setSseConnected] = useState(false)

  return (
    <ConnectionContext.Provider value={{ sseConnected, setSseConnected }}>
      {children}
    </ConnectionContext.Provider>
  )
}

export function useConnection() {
  return useContext(ConnectionContext)
}
