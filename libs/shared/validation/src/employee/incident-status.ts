import { z } from 'zod';

export const incidentStatusOptions = [
  { value: 'reported', label: 'Reported' },
  { value: 'under_investigation', label: 'Under investigation' },
  { value: 'resolved', label: 'Resolved' },
  { value: 'closed', label: 'Closed' },
] as const;

/* Mirrors updateIncident status + corrective_actions on the backend. */
export const incidentStatusSchema = z.object({
  status: z.enum(['reported', 'under_investigation', 'resolved', 'closed'], {
    message: 'Select a status',
  }),
  correctiveActions: z.string(),
});

export type IncidentStatusValues = z.infer<typeof incidentStatusSchema>;
