import { z } from 'zod';
import { createEmailSchemaWithOptions } from '../field-factories';

export const ADMIN_INVITE_DESKS = ['buyer', 'agent', 'hr', 'tech'] as const;

export const createAdminInviteSchema = z.object({
  email: createEmailSchemaWithOptions(),
  desk: z.enum(ADMIN_INVITE_DESKS),
});

export type CreateAdminInviteValues = z.infer<typeof createAdminInviteSchema>;
