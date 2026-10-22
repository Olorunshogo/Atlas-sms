import { z } from 'zod';
import { createEmailSchema, createRequiredString } from '../rules';
import { createNameSchema } from '../field-factories';

export const adminRegisterSchema = z.object({
  email: createEmailSchema(),
  firstName: createNameSchema('First name'),
  lastName: createNameSchema('Last name'),
  password: z
    .string()
    .min(1, 'Password is required')
    .min(8, 'Password must be at least 8 characters'),
  inviteCode: createRequiredString('Invitation code'),
});

export type AdminRegisterValues = z.infer<typeof adminRegisterSchema>;
