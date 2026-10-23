import { useEffect, useState } from 'react';
import { apiFetch, ApiError } from '@atlas/reference-api-client';
import type { RatingTargetType } from '@atlas/reference-ratings';

export interface RatingSubmission {
  id: number;
  score: number;
  facets: string[];
  comment: string | null;
  created_at: string;
}

export interface UseRatingSubmissionsResult {
  submissions: RatingSubmission[];
  loading: boolean;
  error: string | null;
}

/* Admin-only - see RatingsService.listTargetSubmissions. */
export function useRatingSubmissions(
  targetType: RatingTargetType,
  targetId: number | null,
): UseRatingSubmissionsResult {
  const [submissions, setSubmissions] = useState<RatingSubmission[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (targetId === null) {
      setLoading(false);
      return;
    }

    let cancelled = false;
    setLoading(true);
    apiFetch<RatingSubmission[]>(
      `/ratings/target/${targetType}/${targetId}/submissions`,
    )
      .then((data) => {
        if (!cancelled) {
          setSubmissions(data);
          setError(null);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setSubmissions([]);
          setError(
            err instanceof ApiError ? err.message : 'Could not load reviews.',
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

  return { submissions, loading, error };
}
