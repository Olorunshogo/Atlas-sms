import { z } from 'zod';
import { createEmailSchema } from '../rules';
import { createOtpSchema } from '../field-factories';

export const verifyEmailSchema = z.object({
  email: createEmailSchema(),
  otp: createOtpSchema(),
});

export type VerifyEmailValues = z.infer<typeof verifyEmailSchema>;

export const resendOtpSchema = z.object({
  email: createEmailSchema(),
});

export type ResendOtpValues = z.infer<typeof resendOtpSchema>;
