import { STORE_ID_COOKIE } from '../../utils/cookies';

interface SelectStoreBody {
  store_id: string;
}

/**
 * Remembers the store the dashboard is looking at.
 * The id is not a credential. The proxy reads this cookie and sends it as x-store-id. The JWT still decides which tenant the caller is.
 */
export default defineEventHandler(async (event) => {
  const body = await readBody<SelectStoreBody>(event);
  const storeId = body?.store_id?.trim() ?? '';
  if (!storeId) {
    throw createError({
      statusCode: 400,
      statusMessage: 'store_id is required',
    });
  }

  setCookie(event, STORE_ID_COOKIE, storeId, cookieOptions(event));
  return {
    success: true,
    message: 'Store selected.',
    data: { store_id: storeId },
  };
});

function cookieOptions(event: Parameters<typeof setCookie>[0]) {
  return {
    httpOnly: true,
    sameSite: 'lax' as const,
    path: '/',
    secure: getRequestProtocol(event, { xForwardedProto: true }) === 'https',
  };
}
