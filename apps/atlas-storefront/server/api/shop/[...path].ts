import type { H3Event } from 'h3';
import {
  CART_COOKIE,
  CART_MAX_AGE,
  SHOPPER_COOKIE,
  SHOPPER_MAX_AGE,
  isRecord,
  shopUrl,
} from '../../utils/shop';

/**
 * The browser never holds the shopper token or the guest cart token.
 * Both live in HttpOnly cookies; this route turns them back into the headers the shop API expects,
 * and pulls any new token out of a response body into its cookie before the body reaches the page.
 */
export default defineEventHandler(async (event): Promise<unknown> => {
  const { apiBase, shopHandle } = useRuntimeConfig();
  if (!shopHandle) {
    setResponseStatus(event, 500);
    return {
      success: false,
      message: 'ATLAS_SHOP_HANDLE is not set for this storefront.',
    };
  }
  const path = joinedPath(event.context.params?.path);
  const method = event.method;
  const rawBody =
    method === 'GET' || method === 'HEAD'
      ? undefined
      : await readRawBody(event);
  const shopper = getCookie(event, SHOPPER_COOKIE);
  const cart = getCookie(event, CART_COOKIE);

  let response;
  try {
    response = await $fetch.raw(shopUrl(apiBase, shopHandle, path), {
      method,
      query: getQuery(event),
      body: rawBody,
      headers: {
        ...(shopper ? { Authorization: `Bearer ${shopper}` } : {}),
        ...(cart ? { 'x-cart-token': cart } : {}),
        ...(rawBody
          ? {
              'content-type':
                getRequestHeader(event, 'content-type') ?? 'application/json',
            }
          : {}),
      },
      ignoreResponseError: true,
      responseType: 'text',
    });
  } catch {
    setResponseStatus(event, 502);
    return { success: false, message: 'The shop is not reachable.' };
  }

  setResponseStatus(event, response.status);
  const contentType = response.headers.get('content-type') ?? '';
  const text = typeof response._data === 'string' ? response._data : '';
  if (!contentType.includes('application/json')) {
    setResponseHeader(event, 'content-type', contentType || 'text/plain');
    return text;
  }

  const payload: unknown = text ? JSON.parse(text) : null;
  if (response.status === 401 && shopper) {
    deleteCookie(event, SHOPPER_COOKIE, { path: '/' });
  }
  if (isRecord(payload) && payload.success === true && isRecord(payload.data)) {
    keepTokens(event, payload.data);
  }
  return payload;
});

function keepTokens(event: H3Event, data: Record<string, unknown>): void {
  const secure = getRequestProtocol(event) === 'https';
  if (typeof data.token === 'string') {
    setCookie(event, SHOPPER_COOKIE, data.token, {
      httpOnly: true,
      sameSite: 'lax',
      secure,
      path: '/',
      maxAge: SHOPPER_MAX_AGE,
    });
    delete data.token;
    // Signing in merged the guest cart into the account
    deleteCookie(event, CART_COOKIE, { path: '/' });
  }
  if (typeof data.cart_token === 'string') {
    setCookie(event, CART_COOKIE, data.cart_token, {
      httpOnly: true,
      sameSite: 'lax',
      secure,
      path: '/',
      maxAge: CART_MAX_AGE,
    });
    delete data.cart_token;
  }
}

function joinedPath(path: string | string[] | undefined): string {
  if (Array.isArray(path)) {
    return path.join('/');
  }
  return path ?? '';
}
