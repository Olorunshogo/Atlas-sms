import { z } from 'zod';

/* Mirrors createProjectReportSchema on the reference backend (careers-ops.dto.ts). */
export const projectReportSchema = z.object({
  projectName: z.string().min(2, 'Name the project').max(200),
  progressPct: z
    .number({ message: 'Enter progress as a percentage' })
    .int()
    .min(0)
    .max(100),
  completedItems: z.string().max(10000),
  blockers: z.string().max(10000),
  nextSteps: z.string().max(10000),
  submitForReview: z.boolean(),
});

export type ProjectReportValues = z.infer<typeof projectReportSchema>;
