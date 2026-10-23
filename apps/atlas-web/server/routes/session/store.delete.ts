import { STORE_ID_COOKIE } from '../../utils/cookies';

export default defineEventHandler((event) => {
  deleteCookie(event, STORE_ID_COOKIE, { path: '/' });
  return { success: true, message: 'Store cleared.', data: { store_id: null } };
});
