import { z } from 'zod';
import {
  createEmailSchemaWithOptions,
  createNameSchema,
  createRequiredStringWithOptions,
} from '../field-factories';

/* Mirrors createContactSchema on the reference backend (create-contact.dto.ts), so a valid form is never rejected server-side. */
export const supportTicketSchema = z.object({
  fullName: createNameSchema(),
  email: createEmailSchemaWithOptions(),
  message: createRequiredStringWithOptions('Message', { min: 15, max: 1000 }),
});

export type SupportTicketValues = z.infer<typeof supportTicketSchema>;
