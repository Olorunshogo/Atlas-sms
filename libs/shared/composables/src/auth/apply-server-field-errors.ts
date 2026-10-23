/*
 * Vue-adapted replacement for the reference source's React lib/server-errors.ts
 * (applyServerFieldErrors), which set errors on a react-hook-form instance.
 * vee-validate's error API differs (setFieldError(name, message) rather than
 * form.setError), so this is a small local port rather than a shared import -
 * packages/lib/server-errors.ts itself is being converted separately and the
 * two may be worth reconciling later.
 *
 * Expects the backend's validation-failure body to carry
 * `{ errors: Record<string, string[] | string> }`, matching the shape the
 * Atlas Rust backend's validation middleware is expected to return - verify
 * this against the actual error body once the backend's validation error
 * format is confirmed.
 */

interface ServerFieldErrorBody {
  errors?: Record<string, string[] | string>;
}

function extractErrorBody(error: unknown): ServerFieldErrorBody | null {
  if (typeof error !== 'object' || error === null) return null;
  const body = (error as { body?: unknown }).body;
  if (typeof body !== 'object' || body === null) return null;
  return body as ServerFieldErrorBody;
}

/*
 * Generic over the caller's own field-name union, since vee-validate's real
 * setFieldError is narrowed to that form's known fields, not a bare string.
 * The server's error keys are only known to match at runtime, so the cast
 * from the loop's plain `string` to `TField` happens once, here, at that boundary.
 */
export function applyServerFieldErrors<TField extends string>(
  error: unknown,
  setFieldError: (
    field: TField,
    message: string | string[] | undefined,
  ) => void,
): void {
  const body = extractErrorBody(error);
  if (!body?.errors) return;

  for (const [field, messages] of Object.entries(body.errors)) {
    const message = Array.isArray(messages) ? messages[0] : messages;
    if (message) {
      setFieldError(field as TField, message);
    }
  }
}
