import { z } from 'zod';

/* Mirrors createPipSchema on the reference backend (careers-ops.dto.ts). */
export const createPipSchema = z.object({
  employeeUserId: z
    .number({ message: 'Pick an employee' })
    .int()
    .positive('Pick an employee'),
  goals: z.string().min(3, 'Describe the goals'),
  timelines: z.string().min(3, 'Describe the timeline'),
  support: z.string(),
  successCriteria: z.string().min(3, 'Describe what success looks like'),
  dueAt: z.string().min(1, 'Pick a due date'),
});

export type CreatePipValues = z.infer<typeof createPipSchema>;
