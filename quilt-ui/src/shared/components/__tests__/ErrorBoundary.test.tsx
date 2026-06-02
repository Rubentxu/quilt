import { render, screen } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { ErrorBoundary } from '../ErrorBoundary'

// A throw-on-render child used to trip the boundary.
const ThrowError = ({ message = 'Test error' }: { message?: string }) => {
  throw new Error(message)
}

describe('ErrorBoundary', () => {
  // React calls console.error when a child throws during render so the
  // developer can see the stack in DevTools. In tests this is noise.
  let originalError: typeof console.error
  beforeEach(() => {
    originalError = console.error
    console.error = vi.fn()
  })
  afterEach(() => {
    console.error = originalError
  })

  it('renders children when no error is thrown', () => {
    render(
      <ErrorBoundary>
        <div>Hello</div>
      </ErrorBoundary>,
    )
    expect(screen.getByText('Hello')).toBeInTheDocument()
  })

  it('catches errors thrown by descendants and shows the fallback', () => {
    render(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>,
    )
    expect(screen.getByText(/Something went wrong/i)).toBeInTheDocument()
    // The default fallback surfaces the error message verbatim.
    expect(screen.getByText('Test error')).toBeInTheDocument()
  })

  it('renders a "Try again" button in the fallback', () => {
    render(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>,
    )
    expect(
      screen.getByRole('button', { name: /try again/i }),
    ).toBeInTheDocument()
  })

  it('falls back to a generic message when the error has none', () => {
    const NoMessage = () => {
      throw new Error()
    }
    render(
      <ErrorBoundary>
        <NoMessage />
      </ErrorBoundary>,
    )
    // Default copy when error.message is empty.
    expect(screen.getByText(/An unexpected error occurred/i)).toBeInTheDocument()
  })

  it('uses a custom fallback when one is provided', () => {
    render(
      <ErrorBoundary fallback={<div>Custom fallback</div>}>
        <ThrowError />
      </ErrorBoundary>,
    )
    expect(screen.getByText('Custom fallback')).toBeInTheDocument()
    // The default fallback should NOT be present.
    expect(screen.queryByText(/Something went wrong/i)).not.toBeInTheDocument()
  })
})
