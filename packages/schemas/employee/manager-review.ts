import { z } from 'zod';

/* Mirrors managerReviewSchema on the reference backend (careers-ops.dto.ts). */
export const managerReviewSchema = z.object({
  managerGoalsMet: z.boolean(),
  managerRating: z
    .number({ message: 'Rate this review' })
    .int()
    .min(1, 'Must be between 1 and 5')
    .max(5, 'Must be between 1 and 5'),
  managerFeedback: z.string().max(20000),
  probationDecision: z.enum(['confirm', 'terminate', '']),
});

export type ManagerReviewValues = z.infer<typeof managerReviewSchema>;
