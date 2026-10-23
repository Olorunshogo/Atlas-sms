/** Cookie names */
export declare const ACCESS_TOKEN_COOKIE = 'access_token';
export declare const REFRESH_TOKEN_COOKIE = 'refresh_token';
/** Decode JWT payload without verifying signature (client-side only) */
export declare function decodeJwtPayload<T = Record<string, unknown>>(
  token: string,
): T | null;
/**
 * Build Set-Cookie header strings for SSR responses.
 * Use these in React Router actions/loaders via `headers`.
 */
export declare function buildAuthCookieHeaders(
  accessToken: string,
  refreshToken: string,
): string[];
/** Build clear-cookie headers for logout */
export declare function buildClearCookieHeaders(): string[];
/** Parse a cookie string and return the value for a given name */
export declare function parseCookie(
  cookieHeader: string,
  name: string,
): string | null;
