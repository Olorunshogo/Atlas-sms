import { z } from 'zod';
import {
  createPasswordSchema,
  createConfirmPasswordSchema,
  createRequiredString,
  passwordsMatch,
  PASSWORDS_MATCH_ERROR,
} from '../generics';

/*
 * A logged-in user changing a password they already know, unlike
 * password-reset.ts's forgot/reset pair, which is for a user who does not.
 * Reused across every role - the fields and rules are identical regardless of
 * who is asking.
 */
const updatePasswordObject = z.object({
  currentPassword: createRequiredString('Old password'),
  password: createPasswordSchema(),
  confirmPassword: createConfirmPasswordSchema(),
});

export const updatePasswordSchema = updatePasswordObject
  .refine(passwordsMatch, PASSWORDS_MATCH_ERROR)
  .refine((data) => data.currentPassword !== data.password, {
    message: 'New password must be different from your old password',
    path: ['password'],
  });

export type UpdatePasswordValues = z.infer<typeof updatePasswordObject>;
