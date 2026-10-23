import { z } from 'zod';

/* Mirrors updateOutreachTargetsSchema on the reference backend (careers-ops.dto.ts). */
export const outreachTargetsSchema = z.object({
  callWeeklyTarget: z
    .number({ message: 'Enter the weekly call target' })
    .int()
    .min(1, 'Must be at least 1')
    .max(50, 'Must be 50 or fewer'),
  messageWeeklyTarget: z
    .number({ message: 'Enter the weekly message target' })
    .int()
    .min(1, 'Must be at least 1')
    .max(50, 'Must be 50 or fewer'),
});

export type OutreachTargetsValues = z.infer<typeof outreachTargetsSchema>;
