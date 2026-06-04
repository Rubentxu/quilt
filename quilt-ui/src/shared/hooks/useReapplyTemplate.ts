/**
 * useReapplyTemplate — reapply template properties to a block (F15).
 *
 * Mutation hook that wraps POST /api/v1/templates/:name/reapply/:blockId.
 * Exposes loading + error state plus the applied/preserved/overwritten result.
 */

import { useCallback, useState } from 'react';

/** Reapply mode — must match the Rust ReapplyMode enum. */
export type ReapplyMode = 'override_all' | 'preserve_manual';

/** Result of a template reapplication. Mirrors Rust ReapplyResult. */
export interface ReapplyResult {
  applied: string[];
  preserved: string[];
  overwritten: string[];
}

interface UseReapplyTemplateOptions {
  onSuccess?: (result: ReapplyResult) => void;
  onError?: (error: Error) => void;
}

interface UseReapplyTemplateResult {
  reapply: (templateName: string, blockId: string, mode: ReapplyMode) => Promise<ReapplyResult>;
  result: ReapplyResult | null;
  loading: boolean;
  error: string | null;
}

export function useReapplyTemplate(
  options: UseReapplyTemplateOptions = {},
): UseReapplyTemplateResult {
  const { onSuccess, onError } = options;
  const [result, setResult] = useState<ReapplyResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reapply = useCallback(
    async (templateName: string, blockId: string, mode: ReapplyMode): Promise<ReapplyResult> => {
      setLoading(true);
      setError(null);

      try {
        const response = await fetch(`/api/v1/templates/${encodeURIComponent(templateName)}/reapply/${blockId}`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ mode }),
        });

        if (!response.ok) {
          let detail = response.statusText;
          try {
            const body = await response.json();
            detail = body.error || detail;
          } catch {
            // ignore parse error
          }
          const err = new Error(detail);
          setError(detail);
          onError?.(err);
          throw err;
        }

        const reapplyResult: ReapplyResult = await response.json();
        setResult(reapplyResult);
        onSuccess?.(reapplyResult);
        return reapplyResult;
      } catch (err) {
        if (!(err instanceof Error)) {
          const message = String(err);
          setError(message);
          const e = new Error(message);
          onError?.(e);
          throw e;
        }
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [onSuccess, onError],
  );

  return { reapply, result, loading, error };
}
