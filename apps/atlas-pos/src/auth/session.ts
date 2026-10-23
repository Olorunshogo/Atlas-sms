import { reactive } from 'vue';

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
}

export const posSession = reactive<PosSession>({
  accessToken: null,
  refreshToken: null,
  tenantId: null,
  storeId: null,
  userName: '',
});

export function rememberSession(input: {
  accessToken: string;
  refreshToken: string;
  tenantId: string;
  userName: string;
}): void {
  posSession.accessToken = input.accessToken;
  posSession.refreshToken = input.refreshToken;
  posSession.tenantId = input.tenantId;
  posSession.userName = input.userName;
}

export function clearPosSession(): void {
  posSession.accessToken = null;
  posSession.refreshToken = null;
  posSession.tenantId = null;
  posSession.storeId = null;
  posSession.userName = '';
}

export function signedIn(): boolean {
  return posSession.accessToken !== null;
}
