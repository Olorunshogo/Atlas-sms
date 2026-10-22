import { z } from 'zod';
import {
  createEmailSchema,
  createNameSchema,
  createRequiredString,
} from '../generics';

/* Mirrors createContactSchema on the reference backend (create-contact.dto.ts), so a valid form is never rejected server-side. */
export const supportTicketSchema = z.object({
  fullName: createNameSchema(),
  email: createEmailSchema(),
  message: createRequiredString('Message', { min: 15, max: 1000 }),
});

export type SupportTicketValues = z.infer<typeof supportTicketSchema>;
