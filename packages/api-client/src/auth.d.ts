import type { AuthTokens } from './types/auth';
/**
 * Store access and refresh tokens as browser cookies: a 15 minute TTL for the access token, 7 days for the refresh token.
 * Note: HttpOnly cannot be set via JS - for true HttpOnly protection the server must set these via Set-Cookie response headers.
 * This client-side path is used for SPA flows where SSR headers aren't available.
 */
export declare function storeTokens(
  accessToken: string,
  refreshToken: string,
): void;
/** Read the access token from cookies (client-side). */
export declare function getAccessToken(): string | null;
/** Read the refresh token from cookies (client-side). */
export declare function getRefreshToken(): string | null;
/** Clear both tokens by setting Max-Age=0 cookies. */
export declare function clearTokens(): void;
/** Call POST /api/v1/auth/logout then clear stored tokens. */
export declare function logout(): Promise<void>;
/**
 * Attempt a silent token refresh.
 * Concurrent callers share a single in-flight request - see the note above.
 * On 401, clears all tokens and throws so the caller can redirect to /login.
 */
export declare function refreshTokens(): Promise<AuthTokens>;
