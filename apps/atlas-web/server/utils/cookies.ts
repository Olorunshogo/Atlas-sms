export const ACCESS_TOKEN_COOKIE = 'access_token';
export const REFRESH_TOKEN_COOKIE = 'refresh_token';
export const TENANT_ID_COOKIE = 'tenant_id';
export const STORE_ID_COOKIE = 'store_id';

export function extractCookieValue(
  setCookieHeaders: string[],
  name: string,
): string | undefined {
  const header = setCookieHeaders.find((cookie) =>
    cookie.startsWith(`${name}=`),
  );
  return header?.split(';')[0]?.split('=')[1];
}
