import {
  atlasRequest,
  createAtlasClient,
  isSuccess,
  type ApiResult,
} from '@atlas/api';
import { clearPosSession, posSession, rememberSession } from './session';

export type {
  ApiBody,
  ApiFailure,
  ApiResult,
  ApiSuccess,
  SessionProfile,
  SessionUser,
  StoreRecord,
  TenantChoice,
} from '@atlas/api';

export { isSuccess };

interface IssuedSession {
  access_token?: string;
  refresh_token?: string;
  tenant_id: string;
  user: { full_name: string };
}

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

  if (
    !isSuccess(result.body) ||
    !result.body.data.access_token ||
    !result.body.data.refresh_token
  ) {
    clearPosSession();
    return false;
  }

  rememberSession({
    accessToken: result.body.data.access_token,
    refreshToken: result.body.data.refresh_token,
    tenantId: result.body.data.tenant_id,
    userName: result.body.data.user.full_name,
  });
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
  return atlasRequest<T>(posClient, {
    url: path,
    method: init?.method ?? 'GET',
    data: init?.body,
    atlasAuth: init?.auth,
  });
}
