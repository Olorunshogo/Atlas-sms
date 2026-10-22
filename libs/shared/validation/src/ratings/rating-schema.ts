import { z } from 'zod';

/*
 * A post-transaction ratings feature from the reference source. Atlas has no
 * ratings domain. The original import (the reference ratings package's
 * RatingContextConfig) is a workspace package that doesn't exist here;
 * inlined as a local type below. Prune candidate.
 */
export interface RatingContextConfig {
  allowComment: boolean;
}

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
