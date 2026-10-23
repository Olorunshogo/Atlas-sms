import { z } from 'zod';
import {
  createEmailSchemaWithOptions,
  createNameSchema,
  createRequiredStringWithOptions,
} from '../field-factories';

/*
 * Mirrors createContactSchema on the reference backend (create-contact.dto.ts).
 * firstName + lastName are joined to full_name at the submission boundary.
 * subject is required here even though the DTO marks it optional - the marketing form always asks for an inquiry type.
 */
export const marketingContactSchema = z.object({
  firstName: createNameSchema('First name'),
  lastName: createNameSchema('Last name'),
  email: createEmailSchemaWithOptions(),
  subject: z.string().min(1, 'Please select an inquiry type.'),
  message: createRequiredStringWithOptions('Message', { min: 15, max: 1000 }),
});

export type MarketingContactValues = z.infer<typeof marketingContactSchema>;
