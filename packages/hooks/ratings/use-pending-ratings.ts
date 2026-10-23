import { useCallback, useEffect, useState } from 'react';
import { apiFetch, ApiError } from '@atlas/reference-api-client';
import type { RatingContextKey } from '@atlas/reference-ratings';

export interface PendingRating {
  orderId: number;
  contextKey: RatingContextKey;
  title: string;
  description: string;
  expiresAt: string;
}

export interface UsePendingRatingsResult {
  pending: PendingRating[];
  loading: boolean;
  error: string | null;
  reload: () => void;
}

/*
 * Backs a "My ratings" pending tab. The notification bell is the primary surface for a pending rating - this is the fuller list a viewer opens on purpose.
 */
export function usePendingRatings(): UsePendingRatingsResult {
  const [pending, setPending] = useState<PendingRating[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (): Promise<void> => {
    setLoading(true);
    try {
      const rows = await apiFetch<PendingRating[]>('/ratings/pending');
      setPending(rows);
      setError(null);
    } catch (err) {
      setPending([]);
      setError(
        err instanceof ApiError
          ? err.message
          : 'Could not load pending ratings.',
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return { pending, loading, error, reload: () => void load() };
}
