import { z } from 'zod';
import { createNameSchema, createPhoneSchema } from '../generics';

/*
 * `relationship` holds a fixed option value (e.g. "spouse") or the literal
 * "other"; `relationshipOther` is the free-text relationship typed in when
 * "other" is picked, and is only required in that case.
 */
export const emergencyContactSchema = z
  .object({
    name: createNameSchema('Name', { requiredMessage: 'Enter a name' }),
    relationship: z.string().min(1, 'Select a relationship'),
    relationshipOther: z.string(),
    phone: createPhoneSchema({ requiredMessage: 'Enter a phone number' }),
  })
  .refine(
    (data) =>
      data.relationship !== 'other' || data.relationshipOther.trim().length > 0,
    {
      message: 'Enter the relationship',
      path: ['relationshipOther'],
    },
  );

export type EmergencyContactValues = z.infer<typeof emergencyContactSchema>;
