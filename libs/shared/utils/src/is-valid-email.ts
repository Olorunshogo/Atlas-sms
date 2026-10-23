/*
 * Format check only, shared so a footer and a contact form stop carrying their own copies of the same regex.
 * The backend's Zod schema is still the real gate - this exists to give a field an inline error before a request is even sent.
 * For a validated form field, prefer createEmailSchema from @atlas/validation instead - this is for a quick inline check outside a form.
 */
export function isValidEmail(email: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}
