export const SHOPPER_COOKIE = 'atlas_shopper';
export const CART_COOKIE = 'atlas_cart';

// Matches the backend's shopper token and guest cart lifetimes
export const SHOPPER_MAX_AGE = 30 * 24 * 60 * 60;
export const CART_MAX_AGE = 30 * 24 * 60 * 60;

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function shopUrl(apiBase: string, handle: string, path: string): string {
  const base = apiBase.replace(/\/$/, '');
  const rest = path ? `/${path}` : '';
  return `${base}/api/v1/shop/${encodeURIComponent(handle)}${rest}`;
}
