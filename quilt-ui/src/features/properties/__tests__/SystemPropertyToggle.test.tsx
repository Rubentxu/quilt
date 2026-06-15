import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { SystemPropertyToggle } from '../SystemPropertyToggle';

function renderToggle(
  { showSystem = false, onToggle = vi.fn() } = {},
) {
  return {
    onToggle,
    ...render(<SystemPropertyToggle showSystem={showSystem} onToggle={onToggle} />),
  };
}

describe('SystemPropertyToggle', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders with hidden system properties label when showSystem is false', () => {
    renderToggle({ showSystem: false });
    expect(screen.getByText('Show system')).toBeInTheDocument();
  });

  it('renders with hidden system properties label when showSystem is false', () => {
    renderToggle({ showSystem: false });
    expect(screen.getByRole('button', { name: /show system properties/i })).toBeInTheDocument();
  });

  it('renders with hide system properties label when showSystem is true', () => {
    renderToggle({ showSystem: true });
    expect(screen.getByText('Hide system')).toBeInTheDocument();
  });

  it('renders with correct aria-pressed=false when system properties are hidden', () => {
    renderToggle({ showSystem: false });
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'false');
  });

  it('renders with correct aria-pressed=true when system properties are visible', () => {
    renderToggle({ showSystem: true });
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'true');
  });

  it('calls onToggle with true when clicked while hidden', () => {
    const onToggle = vi.fn();
    renderToggle({ showSystem: false, onToggle });
    fireEvent.click(screen.getByRole('button'));
    expect(onToggle).toHaveBeenCalledWith(true);
  });

  it('calls onToggle with false when clicked while visible', () => {
    const onToggle = vi.fn();
    renderToggle({ showSystem: true, onToggle });
    fireEvent.click(screen.getByRole('button'));
    expect(onToggle).toHaveBeenCalledWith(false);
  });
});
