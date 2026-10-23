import { onMounted, ref, type Ref } from 'vue';
import type { RatingContextKey } from './use-rating-submission-service';

/*
 * Backs a "My ratings" pending tab - ported from the reference source's React
 * use-pending-ratings.ts. See use-rating-submission-service.ts's header
 * note: this entire subtree has no Atlas SMS equivalent yet.
 */

export interface PendingRating {
  orderId: number;
  contextKey: RatingContextKey;
  title: string;
  description: string;
  expiresAt: string;
}

export interface UsePendingRatingsResult {
  pending: Ref<PendingRating[]>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
  reload: () => void;
}

export function usePendingRatings(): UsePendingRatingsResult {
  const pending = ref<PendingRating[]>([]);
  const loading = ref<boolean>(true);
  const error = ref<string | null>(null);

  async function load(): Promise<void> {
    loading.value = true;
    try {
      // Replace with an @atlas/api atlasRequest('/ratings/pending') call
      // once this is wired into an actual Atlas app.
      throw new Error('usePendingRatings: wire this to @atlas/api before use.');
    } catch (err) {
      pending.value = [];
      error.value =
        err instanceof Error ? err.message : 'Could not load pending ratings.';
    } finally {
      loading.value = false;
    }
  }

  onMounted(load);

  return { pending, loading, error, reload: () => void load() };
}
