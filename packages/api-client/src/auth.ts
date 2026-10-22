import type { AuthTokens } from './types/auth';
import {
  ACCESS_TOKEN_COOKIE,
  REFRESH_TOKEN_COOKIE,
  buildClearCookieHeaders,
  parseCookie,
} from './auth-cookies';
import { BASE_BACKEND_URL } from './api';

/**
 * Store access and refresh tokens as browser cookies: a 15 minute TTL for the access token, 7 days for the refresh token.
 * Note: HttpOnly cannot be set via JS - for true HttpOnly protection the server must set these via Set-Cookie response headers.
 * This client-side path is used for SPA flows where SSR headers aren't available.
 */
export function storeTokens(accessToken: string, refreshToken: string): void {
  if (typeof document === 'undefined') return;
  document.cookie = `${ACCESS_TOKEN_COOKIE}=${accessToken}; Path=/; Max-Age=900; SameSite=Strict`;
  document.cookie = `${REFRESH_TOKEN_COOKIE}=${refreshToken}; Path=/; Max-Age=604800; SameSite=Strict`;
}

/** Read the access token from cookies (client-side). */
export function getAccessToken(): string | null {
  if (typeof document === 'undefined') return null;
  return parseCookie(document.cookie, ACCESS_TOKEN_COOKIE);
}

/** Read the refresh token from cookies (client-side). */
export function getRefreshToken(): string | null {
  if (typeof document === 'undefined') return null;
  return parseCookie(document.cookie, REFRESH_TOKEN_COOKIE);
}

/** Clear both tokens by setting Max-Age=0 cookies. */
export function clearTokens(): void {
  if (typeof document === 'undefined') return;
  const headers = buildClearCookieHeaders();
  for (const header of headers) {
    document.cookie = header;
  }
}

/** Call POST /api/v1/auth/logout then clear stored tokens. */
export async function logout(): Promise<void> {
  const accessToken = getAccessToken();
  if (accessToken) {
    try {
      await fetch(`${BASE_BACKEND_URL}/auth/logout`, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${accessToken}`,
          'Content-Type': 'application/json',
        },
        credentials: 'include',
      });
    } catch {
      // best-effort - clear tokens even if request fails
    }
  }
  clearTokens();
}

// === Single-flight refresh

/*
 * The backend rotates the refresh token on every use and stores exactly one bcrypt hash per user (auth.service.ts saveRefreshToken).
 * Two concurrent refreshes therefore invalidate each other: the first rotates the stored hash, the second fails bcrypt.compare, gets a 401, and clears the session.
 * Pages that fire parallel requests (agent/overview.tsx does three in one Promise.all) hit this the moment the access token expires.
 *
 * Every caller shares one in-flight request so only one rotation happens.
 * Each caller then retries its own original request with the new token.
 */
let inFlightRefresh: Promise<AuthTokens> | null = null;

/**
 * Attempt a silent token refresh.
 * Concurrent callers share a single in-flight request - see the note above.
 * On 401, clears all tokens and throws so the caller can redirect to /login.
 */
export function refreshTokens(): Promise<AuthTokens> {
  if (inFlightRefresh) return inFlightRefresh;

  inFlightRefresh = performRefresh().finally(() => {
    inFlightRefresh = null;
  });

  return inFlightRefresh;
}

/** Sends the stored refresh token to POST /api/v1/auth/refresh using the `Authorization: Refresh <token>` header. */
async function performRefresh(): Promise<AuthTokens> {
  const refreshToken = getRefreshToken();
  if (!refreshToken) {
    clearTokens();
    throw new Error('No refresh token available');
  }

  const res = await fetch(`${BASE_BACKEND_URL}/auth/refresh`, {
    method: 'POST',
    headers: {
      Authorization: `Refresh ${refreshToken}`,
      'Content-Type': 'application/json',
    },
    credentials: 'include',
  });

  if (res.status === 401) {
    clearTokens();
    throw new Error('Refresh token expired or invalid');
  }

  if (!res.ok) {
    throw new Error(`Token refresh failed with status ${res.status}`);
  }

  const json = await res.json();
  const { accessToken, refreshToken: newRefreshToken } =
    json.data as AuthTokens;

  storeTokens(accessToken, newRefreshToken);
  return { accessToken, refreshToken: newRefreshToken };
}
