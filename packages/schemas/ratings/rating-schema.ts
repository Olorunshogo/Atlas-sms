import { z } from 'zod';
import type { RatingContextConfig } from '@atlas/reference-ratings';

/*
 * Built per context rather than once, since a micro context turns the comment field off entirely.
 * Mirrors createWithdrawalSchema's shape: a factory closing over the one thing that varies.
 */
export function buildRatingSchema(context: RatingContextConfig) {
  return z.object({
    score: z.number({ message: 'Choose a rating' }).int().min(1).max(5),
    facets: z.array(z.string()),
    comment: context.allowComment
      ? z.string().max(2000).optional()
      : z.undefined(),
  });
}

export type RatingValues = z.infer<ReturnType<typeof buildRatingSchema>>;
