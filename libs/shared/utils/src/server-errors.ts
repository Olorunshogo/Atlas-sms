import type { FormContext, Path } from 'vee-validate';

interface ServerFieldError {
  field: string;
  message: string;
}

function toCamelCase(snake: string): string {
  return snake.replace(/_([a-z0-9])/g, (_match, char: string) =>
    char.toUpperCase(),
  );
}

/*
 * Reads the backend's ZodValidationPipe `{ message, errors: [{ field, message }] }`
 * shape off a caught error and camelCases each field name, so any form - vee-validate
 * or a plain ref-backed one - can key its own field-error state the same way.
 * Read structurally rather than importing ApiError, to keep this package out of the
 * transport layer.
 */
export function extractServerFieldErrors(
  error: unknown,
): Record<string, string> {
  const body = (error as { body?: { errors?: ServerFieldError[] } } | null)
    ?.body;
  const errors = body?.errors;
  if (!Array.isArray(errors)) return {};

  const result: Record<string, string> = {};
  for (const { field, message } of errors) {
    result[toCamelCase(field)] = message;
  }
  return result;
}

/*
 * Maps those same field errors onto a vee-validate form context, so a 400 shows up
 * under the right input instead of only as a top banner. Ported from the
 * reference source's react-hook-form version - setError/getValues become setFieldError/values.
 * aliases points a backend field at a differently named form input, for when one
 * input feeds several DTO keys (a single full name split into first_name and
 * last_name).
 * Returns the messages it could not place under any input, so the caller can show
 * them as a banner instead of a generic one.
 */
export function applyServerFieldErrors<T extends Record<string, unknown>>(
  error: unknown,
  form: FormContext<T>,
  aliases: Partial<Record<string, Path<T>>> = {},
): string[] {
  const fieldErrors = extractServerFieldErrors(error);
  const knownFields = new Set(Object.keys(form.values as object));
  const unplaced: string[] = [];

  for (const [field, message] of Object.entries(fieldErrors)) {
    const target: string = aliases[field] ?? field;
    if (knownFields.has(target)) {
      form.setFieldError(target as Path<T>, message);
    } else {
      unplaced.push(message);
    }
  }

  return unplaced;
}
