import { z } from 'zod';
import { createEmailSchema, createRequiredString } from '../rules';
import {
  createPasswordSchemaWithOptions,
  createConfirmPasswordSchema,
  passwordsMatch,
  PASSWORDS_MATCH_ERROR,
} from '../field-factories';

// === Step 1: request a reset link

export const forgotPasswordSchema = z.object({
  email: createEmailSchema(),
});

export type ForgotPasswordValues = z.infer<typeof forgotPasswordSchema>;

// === Step 2: set the new password

const resetPasswordObject = z.object({
  token: createRequiredString('Reset token'),
  password: createPasswordSchemaWithOptions(),
  confirmPassword: createConfirmPasswordSchema(),
});

export const resetPasswordSchema = resetPasswordObject.refine(
  passwordsMatch,
  PASSWORDS_MATCH_ERROR,
);

export type ResetPasswordValues = z.infer<typeof resetPasswordObject>;
