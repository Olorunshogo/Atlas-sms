/*
 * Format check only, shared so the footer and the contact form stop carrying their own copies of the same regex.
 * The backend's Zod schema is still the real gate - this exists to give the field an inline error before a request is even sent.
 */
export function isValidEmail(email: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}
