import type { ComputedRef, Ref } from 'vue';

import { useDialogSubmission, type DialogSubmissionStatus } from '@atlas/ui';

/*
 * Ported from the reference source's React use-rating-submission-service.ts. One endpoint
 * regardless of role, so this talks to the API directly rather than through
 * a per-role adapter - mirrors the deposit/wallet composables' shape.
 *
 * The reference ratings package's RatingContextKey and the ratings domain itself do
 * not exist anywhere in this codebase - this entire ratings/ subtree is
 * that source's marketplace review feature and has no Atlas SMS equivalent.
 * Kept as a mechanical port per the conversion request; almost certainly
 * not something Atlas needs wired up.
 */

export type RatingContextKey = string;

export interface RatingValues {
  score: number;
  facets?: string[];
  comment?: string;
}

export interface UseRatingSubmissionServiceResult {
  submit: (values: RatingValues) => Promise<void>;
  status: Ref<DialogSubmissionStatus>;
  error: Ref<string | null>;
  isSubmitting: ComputedRef<boolean>;
}

export function useRatingSubmissionService(
  contextKey: RatingContextKey,
  orderId: number,
): UseRatingSubmissionServiceResult {
  const { status, error, run, isSubmitting } = useDialogSubmission<void>();

  async function submit(values: RatingValues): Promise<void> {
    // The original posted through the reference API client's apiFetch -
    // replace with an @atlas/api atlasRequest call once ratings is wired
    // into an actual Atlas app, if ever.
    await run(async () => {
      throw new Error(
        'useRatingSubmissionService: wire this to @atlas/api before use - ' +
          JSON.stringify({ contextKey, orderId, ...values }),
      );
    });
  }

  return { submit, status, error, isSubmitting };
}
