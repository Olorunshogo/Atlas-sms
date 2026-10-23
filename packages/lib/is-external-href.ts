/*
 * Whether an href leaves the app and therefore needs a real anchor rather than client-side routing.
 * Protocol-relative URLs (//host) count as external too.
 */
export function isExternalHref(href: string): boolean {
  return /^([a-z][a-z0-9+.-]*:|\/\/)/i.test(href);
}
