import { z } from 'zod';

/* Mirrors upsertWorkReportSchema on the reference backend (people-ops.dto.ts); period_type/period_start/period_end are computed client-side for the always-weekly form, so they aren't fields here. */
export const weeklyReportSchema = z.object({
  tasksCompleted: z.string().min(3, 'Write what you did this week').max(20000),
  challenges: z.string().max(10000),
  nextSteps: z.string().max(10000),
});

export type WeeklyReportValues = z.infer<typeof weeklyReportSchema>;
