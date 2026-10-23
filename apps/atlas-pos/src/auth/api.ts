import {
  atlasRequest,
  createAtlasClient,
  isEmailNotVerified,
  isSuccess,
  type ApiResult,
  type IssuedSession,
} from '@atlas/api';
import { clearPosSession, posSession, rememberSession } from './session';

export type {
  ApiBody,
  ApiFailure,
  ApiResult,
  ApiSuccess,
  IssuedSession,
  SessionProfile,
  SessionUser,
  StoreRecord,
  TenantChoice,
} from '@atlas/api';

export { isSuccess };

const apiBase =
  (import.meta.env.VITE_ATLAS_API_BASE as string | undefined) ??
  'http://localhost:5755';

/**
 * Assigned before any request runs. The refresh closure needs the instance, and the instance needs the closure.
 */
// eslint-disable-next-line prefer-const -- assigned once, after declaration, for the circular reason above
let posClient!: ReturnType<typeof createAtlasClient>;
let refreshInFlight: Promise<boolean> | null = null;

function refreshPosSession(): Promise<boolean> {
  if (refreshInFlight) {
    return refreshInFlight;
  }
  refreshInFlight = performRefresh().finally(() => {
    refreshInFlight = null;
  });
  return refreshInFlight;
}

async function performRefresh(): Promise<boolean> {
  const refreshToken = posSession.refreshToken;
  if (!refreshToken) {
    return false;
  }

  const result = await atlasRequest<IssuedSession>(posClient, {
    url: 'auth/refresh',
    method: 'POST',
    data: { refresh_token: refreshToken },
    atlasAuth: false,
  });

  if (!isSuccess(result.body) || !rememberSession(result.body.data)) {
    clearPosSession();
    return false;
  }
  return true;
}

posClient = createAtlasClient({
  baseURL: `${apiBase}/api/v1`,
  clientType: 'app',
  getAccessToken: () => posSession.accessToken,
  getStoreId: () => posSession.storeId,
  refreshSession: () => refreshPosSession(),
});

export async function posRequest<T>(
  path: string,
  init?: { method?: string; body?: unknown; auth?: boolean },
): Promise<ApiResult<T>> {
  const result = await atlasRequest<T>(posClient, {
    url: path,
    method: init?.method ?? 'GET',
    data: init?.body,
    atlasAuth: init?.auth,
  });
  // App.vue sends the till back to the verify screen when this flips
  if (isEmailNotVerified(result)) {
    posSession.emailVerified = false;
  }
  return result;
}
