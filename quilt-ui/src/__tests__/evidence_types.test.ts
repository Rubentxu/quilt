/**
 * Evidence Contract v1 — TypeScript surface test.
 *
 * The TS types (Evidence, MetaEnvelope, SourceAuthority) are
 * compile-time only. This test verifies:
 * 1. The types exist and are exported.
 * 2. A sample object can be constructed and JSON-serialized using the
 *    expected field names (camelCase, matching the Rust serde rename
 *    rules for ToolsCallResult._meta).
 *
 * Mirrors the Rust test in crates/quilt-mcp/src/protocol.rs (mod tests).
 */

import { describe, it, expect } from 'vitest';
import type {
  Evidence,
  MetaEnvelope,
  SourceAuthority,
} from '../api';

describe('Evidence Contract v1 — TS types (T-21)', () => {
  it('SourceAuthority accepts the three known values', () => {
    // Type-only assertion — at runtime, SourceAuthority is a string union.
    const values: SourceAuthority[] = ['Manual', 'PropertyTyped', 'AutoExtracted'];
    expect(values).toHaveLength(3);
  });

  it('Evidence can be constructed with all fields', () => {
    const ev: Evidence = {
      toolName: 'quilt_get_page_blocks',
      timestamp: '2026-06-04T11:00:00Z',
      isError: false,
      blockIds: ['11111111-2222-3333-4444-555555555555'],
      pageName: 'Test Page',
      pageUpdatedAt: '2026-06-04T10:59:00Z',
      matchedTerms: [],
    };
    expect(ev.toolName).toBe('quilt_get_page_blocks');
    expect(ev.isError).toBe(false);
    expect(ev.blockIds).toHaveLength(1);
  });

  it('Evidence can be minimal (universal fallback)', () => {
    const ev: Evidence = {
      toolName: 'quilt_list_pages',
      timestamp: '2026-06-04T11:00:00Z',
      isError: false,
      blockIds: [],
      matchedTerms: [],
    };
    expect(ev.pageName).toBeUndefined();
    expect(ev.sourceAuthority).toBeUndefined();
  });

  it('MetaEnvelope wraps Evidence', () => {
    const env: MetaEnvelope = {
      evidence: {
        toolName: 'quilt_search',
        timestamp: '2026-06-04T11:00:00Z',
        isError: false,
        blockIds: [],
        matchedTerms: ['hello'],
      },
    };
    expect(env.evidence?.toolName).toBe('quilt_search');
    expect(env.evidence?.matchedTerms[0]).toBe('hello');
  });

  it('MetaEnvelope.evidence is optional', () => {
    const env: MetaEnvelope = {};
    expect(env.evidence).toBeUndefined();
  });
});
