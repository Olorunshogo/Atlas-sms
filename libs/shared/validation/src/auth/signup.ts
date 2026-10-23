import { z } from 'zod';
import { createEmailSchema, createPhoneSchema } from '../rules';
import {
  createNameSchema,
  createPasswordSchemaWithOptions,
  createConfirmPasswordSchema,
  createCvSchema,
  passwordsMatch,
  PASSWORDS_MATCH_ERROR,
} from '../field-factories';

/*
 * The base signup shape, identical for every role. Role-specific fields are
 * deliberately absent - extend with .extend() from a role config rather
 * than declaring a second signup schema.
 */
export const baseSignupFields = {
  fullName: createNameSchema(),
  email: createEmailSchema(),
  phone: createPhoneSchema(),
  password: createPasswordSchemaWithOptions(),
  confirmPassword: createConfirmPasswordSchema(),
};

export const baseSignupObject = z.object(baseSignupFields);

export const signupSchema = baseSignupObject.refine(
  passwordsMatch,
  PASSWORDS_MATCH_ERROR,
);

export type SignupValues = z.infer<typeof baseSignupObject>;

/**
 * Builds a role's signup schema from the base fields plus that role's extras.
 */
export function createRoleSignupSchema<T extends z.ZodRawShape>(extension: T) {
  return baseSignupObject
    .extend(extension)
    .refine(passwordsMatch, PASSWORDS_MATCH_ERROR);
}

// === Optional fields a role config may add

export const referralCodeField = z.string().trim().optional();
export const lgaField = z.string().trim().min(1, 'Select your LGA');
export const addressField = z
  .string()
  .trim()
  .min(5, 'Enter your address, at least 5 characters');

export const acceptedTermsField = z.literal(true, {
  error: 'You must accept the Terms and Conditions',
});

export const cvField = createCvSchema().optional();
