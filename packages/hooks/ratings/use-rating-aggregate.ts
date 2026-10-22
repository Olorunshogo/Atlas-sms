import { useEffect, useState } from 'react';
import { apiFetch, ApiError } from '@atlas/reference-api-client';
import type {
  RatingTargetType,
  ScoreDistribution,
} from '@atlas/reference-ratings';

export interface RatingAggregate {
  targetType: RatingTargetType;
  targetId: number;
  score: string;
  count: number;
  displayable: boolean;
  distribution: ScoreDistribution;
  formulaVersion: number;
}

export interface UseRatingAggregateResult {
  aggregate: RatingAggregate | null;
  loading: boolean;
  error: string | null;
}

/*
 * Backs an agent's own rating summary, and admin's view of either an agent or a buyer.
 * The endpoint enforces who may view which target, so a caller only needs to render whatever comes back.
 */
export function useRatingAggregate(
  targetType: RatingTargetType,
  targetId: number | null,
): UseRatingAggregateResult {
  const [aggregate, setAggregate] = useState<RatingAggregate | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (targetId === null) {
      setLoading(false);
      return;
    }

    let cancelled = false;
    setLoading(true);
    apiFetch<RatingAggregate>(`/ratings/target/${targetType}/${targetId}`)
      .then((data) => {
        if (!cancelled) {
          setAggregate(data);
          setError(null);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setAggregate(null);
          setError(
            err instanceof ApiError ? err.message : 'Could not load ratings.',
          );
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [targetType, targetId]);

  return { aggregate, loading, error };
}
