import { z } from 'zod';

/* Mirrors selfAssessmentSchema on the reference backend (careers-ops.dto.ts). */
export const selfAssessmentSchema = z.object({
  selfAchievements: z
    .string()
    .min(3, 'Write what you achieved this period')
    .max(20000),
  selfLearnings: z.string().max(20000),
  selfGoalsMet: z.string().max(20000),
  selfImprovements: z.string().max(20000),
  selfNextGoals: z.string().max(20000),
});

export type SelfAssessmentValues = z.infer<typeof selfAssessmentSchema>;
