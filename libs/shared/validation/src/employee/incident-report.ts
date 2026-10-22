import { z } from 'zod';

export const incidentCategoryOptions = [
  { value: 'safety', label: 'Safety' },
  { value: 'hr_issue', label: 'HR issue' },
  { value: 'conflict', label: 'Conflict' },
  { value: 'damage', label: 'Damage' },
  { value: 'other', label: 'Other' },
] as const;

/* Mirrors createIncidentSchema on the reference backend (careers-ops.dto.ts). */
export const incidentReportSchema = z.object({
  occurredAt: z.string().min(1, 'Pick when it happened'),
  description: z.string().min(3, 'Describe what happened'),
  peopleInvolved: z.string(),
  impact: z.string(),
  correctiveActions: z.string(),
  category: z.enum(['safety', 'hr_issue', 'conflict', 'damage', 'other'], {
    message: 'Select a category',
  }),
  critical: z.boolean(),
});

export type IncidentReportValues = z.infer<typeof incidentReportSchema>;
