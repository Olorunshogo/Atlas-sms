import { API_VERSION } from '../utils/api';
import {
  ACCESS_TOKEN_COOKIE,
  REFRESH_TOKEN_COOKIE,
  STORE_ID_COOKIE,
  extractCookieValue,
} from '../utils/cookies';

/**
 * The browser never sees access or refresh tokens.
 * This route reads the HttpOnly cookies, injects the Bearer header, forwards to atlas-backend, and copies every Set-Cookie back onto the browser response.
 * On http (local dev) the Secure flag is stripped, because the backend always sets it and a non-https browser would drop the session.
 * x-store-id is the caller's store selection, from the request header or the store cookie. Tenant identity stays inside the JWT.
 */
export default defineEventHandler(async (event) => {
  const path = joinedPath(event.context.params?.path);
  const { apiBase } = useRuntimeConfig();
  const backendBase = apiBase.replace(/\/$/, '');
  const method = event.method;
  const isBodyless = method === 'GET' || method === 'HEAD';
  const rawBody = isBodyless ? undefined : await readRawBody(event);

  const forward = async (accessToken: string | undefined) => {
    const storeId =
      getRequestHeader(event, 'x-store-id') ||
      getCookie(event, STORE_ID_COOKIE);

    return $fetch.raw(`${backendBase}/api/${path}`, {
      method,
      query: getQuery(event),
      body: rawBody,
      headers: {
        ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
        ...(storeId ? { 'x-store-id': storeId } : {}),
        ...(rawBody
          ? {
              'content-type':
                getRequestHeader(event, 'content-type') ?? 'application/json',
            }
          : {}),
      },
      ignoreResponseError: true,
    });
  };

  let response = await forward(getCookie(event, ACCESS_TOKEN_COOKIE));

  if (response.status === 401) {
    const refreshToken = getCookie(event, REFRESH_TOKEN_COOKIE);

    if (refreshToken) {
      const refreshed = await $fetch.raw(
        `${backendBase}/api/${API_VERSION}/auth/refresh`,
        {
          method: 'POST',
          headers: {
            cookie: `${REFRESH_TOKEN_COOKIE}=${refreshToken}`,
            'content-type': 'application/json',
          },
          body: '{}',
          ignoreResponseError: true,
        },
      );

      if (refreshed.status === 200) {
        relaySetCookies(event, refreshed.headers.getSetCookie());
        response = await forward(
          extractCookieValue(
            refreshed.headers.getSetCookie(),
            ACCESS_TOKEN_COOKIE,
          ),
        );
      }
    }
  }

  relaySetCookies(event, response.headers.getSetCookie());
  setResponseStatus(event, response.status);
  return response._data;
});

function joinedPath(path: string | string[] | undefined): string {
  if (Array.isArray(path)) {
    return path.join('/');
  }
  return path ?? '';
}

function relaySetCookies(
  event: Parameters<typeof appendResponseHeader>[0],
  cookies: string[],
): void {
  const secure =
    getRequestProtocol(event, { xForwardedProto: true }) === 'https';
  for (const cookie of cookies) {
    const value = secure ? cookie : cookie.replace(/;\s*Secure/gi, '');
    appendResponseHeader(event, 'set-cookie', value);
  }
}
