import { apiFetch, ApiError } from '@atlas/reference-api-client';
import type { RatingContextKey } from '@atlas/reference-ratings';
import { useDialogSubmission } from '../../lib/dialog/use-dialog-submission';
import type { RatingValues } from '../../schemas/ratings/rating-schema';

/*
 * One endpoint regardless of role, unlike notifications - so this talks to the API directly rather than through a per-role adapter.
 * Mirrors useChangePinService: the hook collects and submits, the server computes and owns the score.
 */

export interface UseRatingSubmissionServiceResult {
  submit: (values: RatingValues) => Promise<void>;
  status: 'idle' | 'submitting' | 'error' | 'success';
  error: string | null;
  isSubmitting: boolean;
}

export function useRatingSubmissionService(
  contextKey: RatingContextKey,
  orderId: number,
): UseRatingSubmissionServiceResult {
  const { status, error, run, isSubmitting } = useDialogSubmission<void>();

  async function submit(values: RatingValues): Promise<void> {
    await run(async () => {
      await apiFetch('/ratings', {
        method: 'POST',
        body: JSON.stringify({ contextKey, orderId, ...values }),
      });
    });
  }

  return { submit, status, error, isSubmitting };
}

/* Re-exported so a caller can narrow a caught error without importing api-client directly. */
export { ApiError };
