import { z } from 'zod';

/*
 * A second, richer set of field factories ported from the reference source's
 * schemas/generics.ts. This OVERLAPS with rules.ts, which was already a
 * partial port of this same file (its own comment says so) - rules.ts's
 * createEmailSchema/createPhoneSchema/createPasswordSchema/
 * createRequiredString take a bare `message` string, while the versions
 * here take an `options` object with separate required/invalid messages.
 * Both are kept, unreconciled, under different names below (suffixed where
 * they'd otherwise collide) - deciding which API wins is a judgment call
 * for whoever wires the first real form, not something to silently pick
 * here.
 */

// === Options

export interface FieldSchemaOptions {
  requiredMessage?: string;
  invalidMessage?: string;
}

export interface StringSchemaOptions extends FieldSchemaOptions {
  min?: number;
  max?: number;
}

// === Email

export function createEmailSchemaWithOptions(
  options: FieldSchemaOptions = {},
): z.ZodString {
  const {
    requiredMessage = 'Email is required',
    invalidMessage = 'Enter a valid email address',
  } = options;

  return z.string().min(1, requiredMessage).email(invalidMessage);
}

// === Generic strings

function messageForMin(label: string, min: number): string {
  return min === 1
    ? `${label} is required`
    : `${label} must be at least ${min} characters`;
}

export function createRequiredStringWithOptions(
  label: string,
  options: StringSchemaOptions = {},
): z.ZodString {
  const { min = 1, max, requiredMessage } = options;

  let schema = z
    .string()
    .min(min, requiredMessage ?? messageForMin(label, min));

  if (max !== undefined) {
    schema = schema.max(max, `${label} must be at most ${max} characters`);
  }

  return schema;
}

/*
 * Every person-name field in the product, so one rule decides what a name
 * is. Min 3 and deliberately no max.
 */
export function createNameSchema(
  label = 'Full name',
  options: FieldSchemaOptions = {},
): z.ZodString {
  return createRequiredStringWithOptions(label, {
    min: 3,
    requiredMessage: options.requiredMessage,
  });
}

// === Phone

/*
 * Accepts the three shapes a Nigerian number reasonably arrives in: local
 * with a leading 0, international with a plus, or international without
 * one. Format-only - normalizeNigerianPhone below produces the single
 * shape every caller actually stores.
 */
const NIGERIA_PHONE_PATTERN = /^(?:0\d{10}|\+?234\d{10})$/;

export function createPhoneSchemaWithOptions(
  options: FieldSchemaOptions = {},
): z.ZodString {
  const {
    requiredMessage = 'Phone number is required',
    invalidMessage = 'Enter a valid Nigerian phone number',
  } = options;

  return z
    .string()
    .min(1, requiredMessage)
    .regex(NIGERIA_PHONE_PATTERN, invalidMessage);
}

/**
 * Normalizes any of the three accepted shapes to the local 11-digit form
 * (leading 0).
 */
export function normalizeNigerianPhone(value: string): string {
  const digits: string = value.replace(/\D/g, '');
  return digits.startsWith('234') && digits.length === 13
    ? `0${digits.slice(3)}`
    : digits;
}

// === Password

export function createPasswordSchemaWithOptions(
  options: FieldSchemaOptions = {},
): z.ZodString {
  const { requiredMessage = 'Password is required' } = options;

  return z
    .string()
    .min(1, requiredMessage)
    .min(8, 'Password must be at least 8 characters')
    .regex(/[A-Z]/, 'Password must contain at least one uppercase letter')
    .regex(/[0-9]/, 'Password must contain at least one number')
    .regex(
      /[^A-Za-z0-9]/,
      'Password must contain at least one special character',
    );
}

export function createConfirmPasswordSchema(): z.ZodString {
  return z.string().min(1, 'Please confirm your password');
}

/*
 * The password === confirmPassword rule is a cross-field concern, applied
 * at the object level by callers via `.refine(passwordsMatch, PASSWORDS_MATCH_ERROR)`.
 */
export interface PasswordPair {
  password?: unknown;
  confirmPassword?: unknown;
}

export function passwordsMatch(data: PasswordPair): boolean {
  return data.password === data.confirmPassword;
}

export const PASSWORDS_MATCH_ERROR: { message: string; path: PropertyKey[] } = {
  message: 'Passwords do not match',
  path: ['confirmPassword'],
};

// === OTP

export function createOtpSchema(length = 6): z.ZodString {
  return z
    .string()
    .min(1, 'Verification code is required')
    .length(length, `Code must be ${length} digits`)
    .regex(/^\d+$/, 'Code must be digits only');
}

export function createBvnSchema(): z.ZodString {
  return z
    .string()
    .min(1, 'BVN is required')
    .length(11, 'BVN must be 11 digits')
    .regex(/^\d+$/, 'BVN must be digits only');
}

// === Word counts

/* The one definition of a word, shared by a live counter and these schemas. */
export function countWords(text: string): number {
  const trimmed: string = text.trim();
  return trimmed === '' ? 0 : trimmed.split(/\s+/).length;
}

export interface WordCountSchemaOptions {
  min: number;
  max: number;
}

export function createWordCountSchema(
  label: string,
  options: WordCountSchemaOptions,
): z.ZodString {
  const { min, max } = options;

  return z
    .string()
    .refine(
      (value) => countWords(value) >= min,
      `${label} must be at least ${min} words`,
    )
    .refine(
      (value) => countWords(value) <= max,
      `${label} must be at most ${max} words`,
    );
}

// === Files

export interface DocumentFileSchemaOptions {
  label: string;
  accept: readonly string[];
  mimeTypes: readonly string[];
  maxSizeMB: number;
}

export const PDF_DOCUMENT = {
  accept: ['.pdf'],
  mimeTypes: ['application/pdf'],
} as const;

export const CV_MAX_SIZE_MB = 2;

export function createDocumentFileSchema(
  options: DocumentFileSchemaOptions,
): z.ZodType<File, File> {
  const { label, accept, mimeTypes, maxSizeMB } = options;
  const formattedTypes: string = accept
    .map((ext) => ext.replace('.', '').toUpperCase())
    .join(' or ');

  return z
    .instanceof(File, { message: `${label} is required` })
    .refine((file) => {
      const name: string = file.name.toLowerCase();
      return (
        mimeTypes.includes(file.type) ||
        accept.some((ext) => name.endsWith(ext))
      );
    }, `${label} must be a ${formattedTypes} file`)
    .refine(
      (file) => file.size <= maxSizeMB * 1024 * 1024,
      `${label} must be ${maxSizeMB}MB or smaller`,
    );
}

export function createCvSchema(): z.ZodType<File, File> {
  return createDocumentFileSchema({
    label: 'CV',
    ...PDF_DOCUMENT,
    maxSizeMB: CV_MAX_SIZE_MB,
  });
}
