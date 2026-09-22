import * as z from 'zod';

/**
 * Reusable zod rule factories - the single source of truth for "what does a
 * valid email/phone/password/etc. look like" across every Klassivox form.
 * Ported from Stayar's schemas/generics.ts pattern (factory functions that
 * take a custom error message, return a zod schema), ahead of any real input
 * field consuming them yet - no page imports this today. Import a rule only
 * where you're actually validating that field; don't import the whole file
 * for one rule.
 *
 * Usage once a form exists:
 * ```ts
 * import { createEmailSchema, createRequiredString } from '@atlasvalidation';
 * const schema = z.object({
 *   email: createEmailSchema(),
 *   firstName: createRequiredString('First name'),
 * });
 * // then wire the schema into vee-validate via @vee-validate/zod's
 * // toTypedSchema(), per docs/API/stayar-api.md's use<Capability>Service
 * // convention.
 * ```
 */

/** Required, valid email. Lowercased so "Foo@Bar.com" and "foo@bar.com" are treated the same. */
export function createEmailSchema(message = 'Invalid email address') {
  return z.string().min(1, { message }).email({ message }).toLowerCase();
}

/**
 * Nigerian-format local phone number: 7-15 digits, optional leading 0
 * (e.g. "09012288798" or "9012288798"). Regex-only, no phone-parsing
 * dependency - Stayar's original also had a stricter `phoneSchema` built on
 * `libphonenumber-js`; that's a real dependency decision (accuracy vs. bundle
 * size) worth making deliberately once a real phone input exists, not
 * silently added here. Strip any leading 0 in the submission layer, not here.
 */
export function createPhoneSchema(message = 'Invalid phone number') {
  return z
    .string()
    .min(1, { message })
    .regex(/^0?[1-9]\d{6,14}$/, { message });
}

/**
 * Password with the standard "strong password" shape: min length, at least
 * one uppercase, one lowercase, one number, one symbol.
 */
export function createPasswordSchema(min = 8, message?: string) {
  return z
    .string()
    .min(min, {
      message: message ?? `Password must be at least ${min} characters`,
    })
    .regex(/[A-Z]/, {
      message: message ?? 'Password must contain at least one uppercase letter',
    })
    .regex(/[a-z]/, {
      message: message ?? 'Password must contain at least one lowercase letter',
    })
    .regex(/[0-9]/, {
      message: message ?? 'Password must contain at least one number',
    })
    .regex(/[^A-Za-z0-9]/, {
      message:
        message ?? 'Password must contain at least one special character',
    });
}

/** Generic required single-line text field (names, titles, short answers). */
export function createRequiredString(fieldName: string, message?: string) {
  return z.string().min(1, { message: message ?? `${fieldName} is required` });
}

/**
 * Longer free-text field (notes, KYC rejection reasons, report-card
 * comments). Not in Stayar's original generics.ts - added for Klassivox
 * since school-domain forms need this more than Stayar's did. `max` guards
 * against unbounded text hitting the database/PDF-generation layer.
 */
export function createLongTextSchema(
  fieldName: string,
  options?: { min?: number; max?: number },
) {
  const min = options?.min ?? 1;
  const max = options?.max ?? 2000;
  return z
    .string()
    .min(min, { message: `${fieldName} is required` })
    .max(max, { message: `${fieldName} must be ${max} characters or fewer` });
}

/**
 * Calendar date field (date of birth, academic year start/end, attendance
 * date). Not in Stayar's original generics.ts (a rental platform had no
 * real need for it) - Klassivox's domain (student DOB, academic terms,
 * attendance) needs this constantly, so it's added here rather than
 * reinvented per form. Expects an ISO date string ("YYYY-MM-DD") since
 * that's what a native `<input type="date">` and most date pickers emit;
 * `min`/`max` are optional ISO-date bounds (e.g. DOB must be in the past).
 */
export function createDateSchema(
  fieldName: string,
  options?: { min?: string; max?: string; message?: string },
) {
  let schema = z.string().regex(/^\d{4}-\d{2}-\d{2}$/, {
    message: options?.message ?? `${fieldName} must be a valid date`,
  });

  if (options?.min) {
    const min = options.min;
    schema = schema.refine((v) => v >= min, {
      message: `${fieldName} must be on or after ${min}`,
    });
  }
  if (options?.max) {
    const max = options.max;
    schema = schema.refine((v) => v <= max, {
      message: `${fieldName} must be on or before ${max}`,
    });
  }
  return schema;
}

/** Fixed-length numeric PIN/OTP (matches `@atlasui-core`'s input-otp component). */
export function createPinSchema(length = 6, message?: string) {
  return z
    .string()
    .length(length, {
      message: message ?? `Code must be exactly ${length} digits`,
    })
    .regex(/^\d+$/, { message: message ?? 'Code must contain only digits' });
}
