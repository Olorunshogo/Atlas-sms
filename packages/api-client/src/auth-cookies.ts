/** Cookie names */
export const ACCESS_TOKEN_COOKIE = 'access_token';
export const REFRESH_TOKEN_COOKIE = 'refresh_token';

/** Decode JWT payload without verifying signature (client-side only) */
export function decodeJwtPayload<T = Record<string, unknown>>(
  token: string,
): T | null {
  try {
    const base64 = token.split('.')[1];
    if (!base64) return null;
    const json = atob(base64.replace(/-/g, '+').replace(/_/g, '/'));
    return JSON.parse(json) as T;
  } catch {
    return null;
  }
}

/**
 * Build Set-Cookie header strings for SSR responses.
 * Use these in React Router actions/loaders via `headers`.
 */
export function buildAuthCookieHeaders(
  accessToken: string,
  refreshToken: string,
): string[] {
  const secure =
    typeof process !== 'undefined' && process.env['NODE_ENV'] === 'production'
      ? '; Secure'
      : '';

  return [
    `${ACCESS_TOKEN_COOKIE}=${accessToken}; HttpOnly; SameSite=Strict; Path=/${secure}; Max-Age=900`,
    `${REFRESH_TOKEN_COOKIE}=${refreshToken}; HttpOnly; SameSite=Strict; Path=/${secure}; Max-Age=604800`,
  ];
}

/** Build clear-cookie headers for logout */
export function buildClearCookieHeaders(): string[] {
  return [
    `${ACCESS_TOKEN_COOKIE}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0`,
    `${REFRESH_TOKEN_COOKIE}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0`,
  ];
}

/** Parse a cookie string and return the value for a given name */
export function parseCookie(cookieHeader: string, name: string): string | null {
  const match = cookieHeader
    .split(';')
    .map((c) => c.trim())
    .find((c) => c.startsWith(`${name}=`));
  return match ? (match.split('=')[1] ?? null) : null;
}
