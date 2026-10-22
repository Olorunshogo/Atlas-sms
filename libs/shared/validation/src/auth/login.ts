import { z } from 'zod';
import { createEmailSchema } from '../rules';

/*
 * Login deliberately does NOT reuse createPasswordSchema. Existing accounts
 * may predate the strength rules, so applying them here would lock valid
 * users out of their own accounts. Login only checks presence; the server
 * decides.
 */
export const loginSchema = z.object({
  email: createEmailSchema(),
  password: z.string().min(1, 'Password is required'),
});

export type LoginValues = z.infer<typeof loginSchema>;
