import { SHOPPER_COOKIE } from '../../utils/shop';

// Shopper tokens aren't revocable server-side yet, so signing out forgets this browser's copy
export default defineEventHandler((event) => {
  deleteCookie(event, SHOPPER_COOKIE, { path: '/' });
  return { success: true, message: 'Signed out.', data: null };
});
