import { tryUseNuxtApp, useRequestHeaders, useState, type NuxtApp } from '#app';
import {
  atlasRequest,
  createAtlasClient,
  createFetchAdapter,
  isEmailNotVerified,
  isSuccess,
  type ApiResult,
  type InProcessFetch,
  type SessionProfile,
} from '@atlas/api';

export type {
  ApiBody,
  ApiFailure,
  ApiResult,
  ApiSuccess,
  SessionProfile,
  SessionUser,
  StoreRecord,
  TenantChoice,
  TenantRecord,
} from '@atlas/api';

export { isSuccess };

export const SESSION_STATE_KEY = 'atlas-session';

const marketingClient = createAtlasClient({
  baseURL: '/api',
  clientType: 'web',
  withCredentials: true,
  ...(import.meta.server ? { adapter: createFetchAdapter(nitroFetch) } : {}),
});

export async function apiRequest<T>(
  path: string,
  init?: { method?: string; body?: unknown },
): Promise<ApiResult<T>> {
  // Taken before the first await, while the Nuxt context is still set
  const nuxtApp = tryUseNuxtApp();
  const result = await atlasRequest<T>(marketingClient, {
    url: path,
    method: init?.method ?? 'GET',
    data: init?.body,
  });
  if (nuxtApp && isEmailNotVerified(result)) {
    flagSessionUnverified(nuxtApp);
  }
  return result;
}

/**
 * Any tenant call refused for an unverified account flips the session, so the dashboard gate shows
 * even if the profile was loaded before the account changed.
 */
function flagSessionUnverified(nuxtApp: NuxtApp): void {
  nuxtApp.runWithContext(() => {
    const profile = useState<SessionProfile | null>(SESSION_STATE_KEY);
    if (profile.value?.user.email_verified) {
      profile.value = {
        ...profile.value,
        user: { ...profile.value.user, email_verified: false },
      };
    }
  });
}

/**
 * `$fetch` on the server reaches Nitro without a loopback socket.
 * `useRequestFetch()` is typed without `.raw`, and the status code has to survive, so the cookie is copied onto `$fetch.raw` instead.
 */
function nitroFetch(): InProcessFetch {
  return {
    raw: async (url, init) => {
      const headers = { ...init.headers };
      const incoming = useRequestHeaders(['cookie']);
      if (incoming.cookie) {
        headers.cookie = incoming.cookie;
      }
      const response = await globalThis.$fetch.raw(url, {
        method: init.method as
          | 'GET'
          | 'POST'
          | 'PUT'
          | 'PATCH'
          | 'DELETE'
          | 'HEAD',
        headers,
        body: init.body,
        ignoreResponseError: true,
      });
      return {
        _data: response._data,
        status: response.status,
        statusText: response.statusText,
      };
    },
  };
}
