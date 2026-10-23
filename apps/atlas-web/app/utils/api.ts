import { useRequestHeaders } from '#app';
import {
  atlasRequest,
  createAtlasClient,
  createFetchAdapter,
  isSuccess,
  type ApiResult,
  type InProcessFetch,
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
  return atlasRequest<T>(marketingClient, {
    url: path,
    method: init?.method ?? 'GET',
    data: init?.body,
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
