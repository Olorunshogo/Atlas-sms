import { z } from 'zod';
import { createEmailSchema } from '../generics';

export const ADMIN_INVITE_DESKS = ['buyer', 'agent', 'hr', 'tech'] as const;

export const createAdminInviteSchema = z.object({
  email: createEmailSchema(),
  desk: z.enum(ADMIN_INVITE_DESKS),
});

export type CreateAdminInviteValues = z.infer<typeof createAdminInviteSchema>;
