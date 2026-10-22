import { z } from 'zod';
import {
  createEmailSchema,
  createPasswordSchema,
  createConfirmPasswordSchema,
  createRequiredString,
  passwordsMatch,
  PASSWORDS_MATCH_ERROR,
} from '../generics';

// === Step 1: request a reset link

export const forgotPasswordSchema = z.object({
  email: createEmailSchema(),
});

export type ForgotPasswordValues = z.infer<typeof forgotPasswordSchema>;

// === Step 2: set the new password

/*
 * Unlike login, this DOES apply the full strength rules - the user is choosing
 * a new password, so it must satisfy what the backend will accept.
 */
const resetPasswordObject = z.object({
  token: createRequiredString('Reset token'),
  password: createPasswordSchema(),
  confirmPassword: createConfirmPasswordSchema(),
});

export const resetPasswordSchema = resetPasswordObject.refine(
  passwordsMatch,
  PASSWORDS_MATCH_ERROR,
);

export type ResetPasswordValues = z.infer<typeof resetPasswordObject>;
