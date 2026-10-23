import { reactive } from 'vue';
import type { IssuedSession } from '@atlas/api';

/**
 * Access token stays in memory. Refresh token stays in memory too, because this app has no secure-storage plugin installed yet.
 * A reload asks the cashier to log in again. Do not move either token into localStorage.
 */
export interface PosSession {
  accessToken: string | null;
  refreshToken: string | null;
  tenantId: string | null;
  storeId: string | null;
  userName: string;
  email: string;
  /** `false` keeps the till on the verify screen; every tenant call is refused until then. */
  emailVerified: boolean;
}

export const posSession = reactive<PosSession>({
  accessToken: null,
  refreshToken: null,
  tenantId: null,
  storeId: null,
  userName: '',
  email: '',
  emailVerified: false,
});

/** Stores a login, refresh or switch-tenant result. `false` when the body had no tokens, which an `app` client always gets. */
export function rememberSession(issued: IssuedSession): boolean {
  if (!issued.access_token || !issued.refresh_token) {
    return false;
  }
  posSession.accessToken = issued.access_token;
  posSession.refreshToken = issued.refresh_token;
  posSession.tenantId = issued.tenant_id;
  posSession.userName = issued.user.full_name;
  posSession.email = issued.user.email;
  posSession.emailVerified = issued.user.email_verified;
  return true;
}

export function clearPosSession(): void {
  posSession.accessToken = null;
  posSession.refreshToken = null;
  posSession.tenantId = null;
  posSession.storeId = null;
  posSession.userName = '';
  posSession.email = '';
  posSession.emailVerified = false;
}

export function signedIn(): boolean {
  return posSession.accessToken !== null;
}
