import { onMounted, ref, watch, type Ref } from 'vue';
import type { RatingTargetType } from './use-rating-aggregate';

/*
 * Admin-only list of a target's rating submissions - ported from the reference source's
 * React use-rating-submissions.ts. See use-rating-submission-service.ts's
 * header note on relevance to Atlas.
 */

export interface RatingSubmission {
  id: number;
  score: number;
  facets: string[];
  comment: string | null;
  created_at: string;
}

export interface UseRatingSubmissionsResult {
  submissions: Ref<RatingSubmission[]>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
}

export function useRatingSubmissions(
  targetType: RatingTargetType,
  targetId: number | null,
): UseRatingSubmissionsResult {
  const submissions = ref<RatingSubmission[]>([]);
  const loading = ref<boolean>(true);
  const error = ref<string | null>(null);

  async function load(): Promise<void> {
    if (targetId === null) {
      loading.value = false;
      return;
    }

    loading.value = true;
    try {
      // Replace with an @atlas/api atlasRequest(`/ratings/target/${targetType}/${targetId}/submissions`)
      // call once this is wired into an actual Atlas app.
      throw new Error(
        'useRatingSubmissions: wire this to @atlas/api before use.',
      );
    } catch (err) {
      submissions.value = [];
      error.value =
        err instanceof Error ? err.message : 'Could not load reviews.';
    } finally {
      loading.value = false;
    }
  }

  onMounted(load);
  watch([() => targetType, () => targetId], load);

  return { submissions, loading, error };
}
