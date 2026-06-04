/**
 * useSchemaPack — fetch and cache schema pack for a template (G6).
 *
 * Fetches GET /api/v1/templates/:name/schema-pack and caches the result
 * by template name to avoid redundant fetches.
 */

import { useCallback, useEffect, useState } from 'react';
import { api } from '@core/api-client';
import type { SchemaPack } from '@shared/types/schemaPack';

interface UseSchemaPackResult {
  schemaPack: SchemaPack | null;
  loading: boolean;
  error: string | null;
  refetch: () => void;
}

const SCHEMA_PACK_CACHE = new Map<string, SchemaPack | null>();

export function useSchemaPack(templateName: string): UseSchemaPackResult {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [schemaPack, setSchemaPack] = useState<SchemaPack | null>(() => {
    return SCHEMA_PACK_CACHE.get(templateName) ?? null;
  });

  const fetchSchemaPack = useCallback(async () => {
    if (!templateName) return;

    // Return from cache if available
    if (SCHEMA_PACK_CACHE.has(templateName)) {
      setSchemaPack(SCHEMA_PACK_CACHE.get(templateName) ?? null);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const response = await api.getSchemaPack(templateName);
      const pack = (response.schema_pack as SchemaPack | null) ?? null;
      SCHEMA_PACK_CACHE.set(templateName, pack);
      setSchemaPack(pack);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load schema pack';
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [templateName]);

  useEffect(() => {
    fetchSchemaPack();
  }, [fetchSchemaPack]);

  return {
    schemaPack,
    loading,
    error,
    refetch: fetchSchemaPack,
  };
}
