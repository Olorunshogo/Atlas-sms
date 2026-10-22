import { z } from 'zod';
import { createRequiredString } from '../rules';
import {
  createPasswordSchemaWithOptions,
  createConfirmPasswordSchema,
  passwordsMatch,
  PASSWORDS_MATCH_ERROR,
} from '../field-factories';

/*
 * A logged-in user changing a password they already know, unlike
 * password-reset.ts's forgot/reset pair, which is for a user who does not.
 */
const updatePasswordObject = z.object({
  currentPassword: createRequiredString('Old password'),
  password: createPasswordSchemaWithOptions(),
  confirmPassword: createConfirmPasswordSchema(),
});

export const updatePasswordSchema = updatePasswordObject
  .refine(passwordsMatch, PASSWORDS_MATCH_ERROR)
  .refine((data) => data.currentPassword !== data.password, {
    message: 'New password must be different from your old password',
    path: ['password'],
  });

export type UpdatePasswordValues = z.infer<typeof updatePasswordObject>;
