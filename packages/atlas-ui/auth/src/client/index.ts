import { inject, type App, type InjectionKey } from 'vue';
import type { ApiResult } from '@atlas/api';

export interface AuthRequestInit {
  method?: string;
  body?: unknown;
}

/**
 * Sends a request to an `/api/v1`-relative path such as `auth/login`, through whatever transport the host app uses:
 * the Nitro cookie proxy on web, bearer tokens on POS.
 */
export type AuthRequest = <T>(
  path: string,
  init?: AuthRequestInit,
) => Promise<ApiResult<T>>;

export interface AuthClient {
  request: AuthRequest;
}

export const AUTH_CLIENT_KEY: InjectionKey<AuthClient> =
  Symbol('atlas-auth-client');

export function installAuthClient(app: App, client: AuthClient): void {
  app.provide(AUTH_CLIENT_KEY, client);
}

export function useAuthClient(): AuthClient {
  const client = inject(AUTH_CLIENT_KEY, null);
  if (!client) {
    throw new Error(
      'No auth client provided. Call installAuthClient(app, client) at startup.',
    );
  }
  return client;
}
