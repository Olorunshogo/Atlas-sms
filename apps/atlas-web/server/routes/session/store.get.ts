import { STORE_ID_COOKIE } from '../../utils/cookies';

export default defineEventHandler((event) => {
  const storeId = getCookie(event, STORE_ID_COOKIE) ?? null;
  return {
    success: true,
    message: 'Store selection.',
    data: { store_id: storeId },
  };
});
