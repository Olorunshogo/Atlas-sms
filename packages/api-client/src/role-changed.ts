// === Role change signal

// A session whose role was changed server-side, such as an applicant whose offer was accepted.
// The transport raises this instead of a plain "session expired" so the app can celebrate rather than bounce to /login.
export type HiredRole = 'employee' | 'customer_care' | 'tech';

const ROLE_CHANGED_EVENT = 'atlas:role-changed';

export function announceRoleChanged(role: string | undefined): void {
  if (typeof window === 'undefined') return;
  window.dispatchEvent(
    new CustomEvent<string | undefined>(ROLE_CHANGED_EVENT, { detail: role }),
  );
}

// Returns an unsubscribe function.
export function onRoleChanged(
  handler: (role: string | undefined) => void,
): () => void {
  if (typeof window === 'undefined') return () => undefined;
  const listener = (event: Event): void =>
    handler((event as CustomEvent<string | undefined>).detail);
  window.addEventListener(ROLE_CHANGED_EVENT, listener);
  return () => window.removeEventListener(ROLE_CHANGED_EVENT, listener);
}
