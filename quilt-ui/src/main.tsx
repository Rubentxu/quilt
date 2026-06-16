import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from '@tanstack/react-router'
import { WasmProvider } from './core/wasm-bridge/WasmProvider'
import { Toaster } from 'react-hot-toast'
import { ConnectionProvider } from '@shared/contexts/ConnectionContext'
import { TabsProvider } from '@shared/contexts/TabsContext'
import { ErrorBoundary } from '@shared/components/ErrorBoundary'
import { CommandRegistryProvider } from '@features/command-center/context'
import { PanelVisibilityProvider } from '@features/dashboard'
import { FocusModeProvider } from '@features/focus-mode/FocusModeContext'
import { FocusModeToggle } from '@features/focus-mode/FocusModeToggle'
import { router } from './router'
import './globals.css'

// Initialize theme from localStorage
const savedTheme = localStorage.getItem('quilt-theme')
if (savedTheme) {
  document.documentElement.setAttribute('data-theme', savedTheme)
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <WasmProvider>
      <ConnectionProvider>
        <TabsProvider>
          <CommandRegistryProvider>
            <PanelVisibilityProvider>
              <FocusModeProvider>
                <FocusModeToggle />
                <ErrorBoundary>
                  <RouterProvider router={router} />
                  <Toaster
                    position="bottom-right"
                    toastOptions={{
                      style: {
                        background: 'var(--color-surface)',
                        color: 'var(--color-text-primary)',
                        border: '1px solid var(--color-border)',
                        borderRadius: 'var(--radius-md)',
                        fontSize: '14px',
                      },
                    }}
                  />
                </ErrorBoundary>
              </FocusModeProvider>
            </PanelVisibilityProvider>
          </CommandRegistryProvider>
        </TabsProvider>
      </ConnectionProvider>
    </WasmProvider>
  </StrictMode>,
)
