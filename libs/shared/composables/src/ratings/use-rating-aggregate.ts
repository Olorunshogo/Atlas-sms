import { onMounted, ref, watch, type Ref } from 'vue';

/*
 * Backs an agent's own rating summary, and admin's view of either an agent
 * or a buyer - ported from the reference source's React use-rating-aggregate.ts. See
 * use-rating-submission-service.ts's header note on relevance to Atlas.
 */

export type RatingTargetType = string;
export type ScoreDistribution = Record<string, number>;

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
  aggregate: Ref<RatingAggregate | null>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
}

export function useRatingAggregate(
  targetType: RatingTargetType,
  targetId: number | null,
): UseRatingAggregateResult {
  const aggregate = ref<RatingAggregate | null>(null);
  const loading = ref<boolean>(true);
  const error = ref<string | null>(null);

  async function load(): Promise<void> {
    if (targetId === null) {
      loading.value = false;
      return;
    }

    loading.value = true;
    try {
      // Replace with an @atlas/api atlasRequest(`/ratings/target/${targetType}/${targetId}`)
      // call once this is wired into an actual Atlas app.
      throw new Error(
        'useRatingAggregate: wire this to @atlas/api before use.',
      );
    } catch (err) {
      aggregate.value = null;
      error.value =
        err instanceof Error ? err.message : 'Could not load ratings.';
    } finally {
      loading.value = false;
    }
  }

  onMounted(load);
  watch([() => targetType, () => targetId], load);

  return { aggregate, loading, error };
}
