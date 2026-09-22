import { API_VERSION } from '../utils/api';
import {
  ACCESS_TOKEN_COOKIE,
  REFRESH_TOKEN_COOKIE,
  TENANT_ID_COOKIE,
  STORE_ID_COOKIE,
  extractCookieValue,
} from '../utils/cookies';

/**
 * Same proxy shape as atlas-web's: the frontend never sees tokens,
 * only HttpOnly cookies. This route reads them, injects the Bearer header,
 * forwards to atlas-backend, and retries once on a 401 via the refresh
 * token. Runs as a Cloudflare Pages Function via Nitro.
 *
 * Unlike atlas-web, Atlas has no per-tenant subdomain (Klassivox's
 * host-only cookie scoping is a child-safety decision specific to schools,
 * not something Atlas's PRD asks for) - x-tenant-id/x-store-id are read
 * from cookies set at login instead of derived from the host. Once JWT
 * verification lands on atlas-backend, these become claims read off the
 * verified token rather than trusted cookies, same evolution path
 * documented on atlas-backend's TenantTx extractor.
 */
export default defineEventHandler(async (event) => {
  const path = event.context.params?.path ?? '';
  const { apiBase } = useRuntimeConfig();

  const forward = async (accessToken: string | undefined) => {
    const method = event.method;
    const isBodyless = method === 'GET' || method === 'HEAD';

    return $fetch.raw(`${apiBase}/${path}`, {
      method,
      query: getQuery(event),
      body: isBodyless ? undefined : await readRawBody(event),
      headers: {
        ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
        'x-tenant-id': getCookie(event, TENANT_ID_COOKIE) ?? '',
        ...(getCookie(event, STORE_ID_COOKIE)
          ? { 'x-store-id': getCookie(event, STORE_ID_COOKIE)! }
          : {}),
        'content-type':
          getRequestHeader(event, 'content-type') ?? 'application/json',
      },
      ignoreResponseError: true,
    });
  };

  let response = await forward(getCookie(event, ACCESS_TOKEN_COOKIE));

  if (response.status === 401) {
    const refreshToken = getCookie(event, REFRESH_TOKEN_COOKIE);

    if (refreshToken) {
      const refreshed = await $fetch.raw(`${apiBase}/${API_VERSION}/auth/refresh`, {
        method: 'POST',
        headers: { cookie: `${REFRESH_TOKEN_COOKIE}=${refreshToken}` },
        ignoreResponseError: true,
      });

      if (refreshed.status === 200) {
        const setCookies = refreshed.headers.getSetCookie();
        setCookies.forEach((cookie) =>
          appendResponseHeader(event, 'set-cookie', cookie),
        );
        response = await forward(
          extractCookieValue(setCookies, ACCESS_TOKEN_COOKIE),
        );
      }
    }
  }

  setResponseStatus(event, response.status);
  return response._data;
});
