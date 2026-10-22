import { z } from 'zod';

/* Mirrors createPerformanceReviewSchema on the reference backend (careers-ops.dto.ts). */
export const createReviewSchema = z.object({
  employeeUserId: z
    .number({ message: 'Pick an employee' })
    .int()
    .positive('Pick an employee'),
  reviewType: z.enum(['annual', 'probation']),
  dueAt: z.string().min(1, 'Pick a due date'),
});

export type CreateReviewValues = z.infer<typeof createReviewSchema>;
