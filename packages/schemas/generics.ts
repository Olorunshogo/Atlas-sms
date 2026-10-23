import { z } from 'zod';

/*
 * Reusable field-level schema factories. Compose these into whole-form schemas
 * under schemas/<feature>/ instead of redeclaring field rules per form.
 *
 * These are factories rather than bare exported schemas so one rule can serve
 * many forms while each caller still controls its own copy. Copy is the thing
 * that legitimately varies between forms; the rule is not.
 */

// === Options

/*
 * requiredMessage overrides the default "required" message.
 * invalidMessage overrides the default format/validity message.
 */
export interface FieldSchemaOptions {
  requiredMessage?: string;
  invalidMessage?: string;
}

export interface StringSchemaOptions extends FieldSchemaOptions {
  min?: number;
  max?: number;
}

// === Email

export function createEmailSchema(
  options: FieldSchemaOptions = {},
): z.ZodString {
  const {
    requiredMessage = 'Email is required',
    invalidMessage = 'Enter a valid email address',
  } = options;

  return z.string().min(1, requiredMessage).email(invalidMessage);
}

// === Generic strings

export function createRequiredString(
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
 * Every person-name field in the product, so one rule decides what a name is.
 * Min 3 and deliberately no max: the columns are `text`, and a cap only ever rejects a real, long name.
 */
export function createNameSchema(
  label = 'Full name',
  options: FieldSchemaOptions = {},
): z.ZodString {
  return createRequiredString(label, {
    min: 3,
    requiredMessage: options.requiredMessage,
  });
}

// === Phone

/*
 * Accepts the three shapes a Nigerian number reasonably arrives in: local with
 * a leading 0 (09012345678), international with a plus (+2349012345678), or
 * international without one (2349012345678). Deliberately not using
 * libphonenumber-js: it is not a dependency of this package, and the backend
 * accepts a plain string. If real international parsing is ever needed, change
 * it here and every form inherits.
 *
 * Format-only. normalizeNigerianPhone below produces the single shape every
 * caller actually stores, so the schema does not also carry a transform.
 */
const NIGERIA_PHONE_PATTERN = /^(?:0\d{10}|\+?234\d{10})$/;

export function createPhoneSchema(
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
 * (leading 0), so every consumer downstream works with one shape regardless of
 * what the user typed.
 */
export function normalizeNigerianPhone(value: string): string {
  const digits: string = value.replace(/\D/g, '');
  return digits.startsWith('234') && digits.length === 13
    ? `0${digits.slice(3)}`
    : digits;
}

// === Password

/*
 * These four rules mirror `passwordRule` in the reference backend (register.dto.ts) exactly, including
 * message wording. They previously disagreed: the frontend only checked
 * min(8), so a password could pass client validation and be rejected by the
 * server. Keep the two in sync - this is the only reason this factory exists
 * rather than an inline min(8) per form.
 */
export function createPasswordSchema(
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

/**
 * A plain string for the "confirm password" field. The equality rule is a
 * cross-field concern, so it is applied at the object level by
 * withPasswordConfirmation, not here.
 */
export function createConfirmPasswordSchema(): z.ZodString {
  return z.string().min(1, 'Please confirm your password');
}

/*
 * The password === confirmPassword rule is a cross-field concern, so callers
 * apply it themselves with `.refine(passwordsMatch, PASSWORDS_MATCH_ERROR)`.
 * Exported as a predicate plus an error object rather than a schema wrapper
 * because Zod 4's refine() return type makes a generic wrapper more trouble
 * than the duplication it saves.
 *
 * The parameter is intentionally loose (optional, unknown) so this stays
 * assignable to the refine callback of any object schema that extends the base
 * signup shape, whose inferred type Zod does not expose in a narrowable form.
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

/* The one definition of a word, shared by TextareaField's live counter and the schemas, so the two can never disagree about a count. */
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

/*
 * accept lists extensions with the dot (".pdf") and mimeTypes the matching MIME types.
 * Both are checked because a browser reports an empty or generic MIME type for some files, so the extension is the fallback, not the only test.
 */
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

/*
 * The one CV rule, shared by careers applications and agent signup.
 * The backend enforces the same limit and type, so a file passing here is never rejected there.
 */
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

// === Helpers

function messageForMin(label: string, min: number): string {
  return min === 1
    ? `${label} is required`
    : `${label} must be at least ${min} characters`;
}
