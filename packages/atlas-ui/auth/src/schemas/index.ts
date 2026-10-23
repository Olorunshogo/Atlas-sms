import { z } from 'zod';

/**
 * Atlas's own auth payloads (snake_case, code-based reset), not the
 * camelCase marketplace schemas in `@atlas/validation`'s `auth` folder.
 */
const email = z
  .string()
  .min(1, 'Email is required')
  .email('Enter a valid email');
const newPassword = z.string().min(8, 'Password must be at least 8 characters');
const code = z.string().regex(/^\d{6}$/, 'Enter the 6-digit code');
const phone = z.string().min(7, 'Enter a phone number');
const fullName = z.string().trim().min(1, 'Name is required');

// Login only checks presence: the server decides whether a password is right
export const loginSchema = z.object({
  email,
  password: z.string().min(1, 'Password is required'),
  tenant_id: z.string().optional(),
});

export const signupSchema = z.object({
  full_name: fullName,
  tenant_name: z.string().trim().min(1, 'Business name is required'),
  email,
  phone,
  password: newPassword,
});

export const verifyEmailSchema = z.object({ email, code });

export const forgotPasswordSchema = z.object({ email });

export const resetPasswordSchema = z.object({
  email,
  code,
  new_password: newPassword,
});

export const acceptInviteSchema = z.object({
  full_name: fullName,
  phone,
  password: newPassword,
});

export type LoginValues = z.infer<typeof loginSchema>;
export type SignupValues = z.infer<typeof signupSchema>;
export type VerifyEmailValues = z.infer<typeof verifyEmailSchema>;
export type ForgotPasswordValues = z.infer<typeof forgotPasswordSchema>;
export type ResetPasswordValues = z.infer<typeof resetPasswordSchema>;
export type AcceptInviteValues = z.infer<typeof acceptInviteSchema>;
