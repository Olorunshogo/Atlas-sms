import axios, {
  AxiosHeaders,
  type AxiosAdapter,
  type AxiosInstance,
  type AxiosResponse,
  type InternalAxiosRequestConfig,
} from 'axios';
import { normalizeBody } from './envelope';
import type { ApiResult } from './types';

const UNREACHABLE_MESSAGE = 'Atlas API is not reachable.';

interface AtlasFlags {
  atlasAuth?: boolean;
  atlasRetried?: boolean;
}

export interface AtlasClientOptions {
  baseURL: string;
  clientType: 'web' | 'app';
  withCredentials?: boolean;
  adapter?: AxiosAdapter;
  getAccessToken?: () => string | null;
  getStoreId?: () => string | null;
  /**
   * Runs once after a 401 on an authenticated call.
   * Return true when a new access token is stored, so the original request can be sent again.
   * Marketing leaves this unset: the Nitro proxy refreshes the cookie session itself.
   */
  refreshSession?: () => Promise<boolean>;
}

export interface AtlasRequestConfig {
  url: string;
  method?: string;
  data?: unknown;
  baseURL?: string;
  headers?: Record<string, string>;
  atlasAuth?: boolean;
}

type FlaggedConfig = InternalAxiosRequestConfig & AtlasFlags;

/**
 * One Axios instance for every Atlas app.
 * The request interceptor attaches client type, bearer token, and store id.
 * The response interceptor always settles with `{ status, body }`, including HTTP errors and network failures.
 */
export function createAtlasClient(options: AtlasClientOptions): AxiosInstance {
  const instance = axios.create({
    baseURL: options.baseURL,
    withCredentials: options.withCredentials ?? false,
    ...(options.adapter ? { adapter: options.adapter } : {}),
  });

  instance.interceptors.request.use((config) => {
    const flagged = config as FlaggedConfig;
    flagged.headers = AxiosHeaders.from(flagged.headers);
    flagged.headers.set('x-client-type', options.clientType);

    if (flagged.atlasAuth !== false) {
      const token = options.getAccessToken?.() ?? null;
      if (token) {
        flagged.headers.set('Authorization', `Bearer ${token}`);
      }
    }

    const storeId = options.getStoreId?.() ?? null;
    if (storeId) {
      flagged.headers.set('x-store-id', storeId);
    }

    return flagged;
  });

  instance.interceptors.response.use(
    (response) =>
      normalizeResult(
        response.status,
        response.data,
      ) as unknown as AxiosResponse,
    async (error: unknown) => {
      const flagged = axios.isAxiosError(error)
        ? (error.config as FlaggedConfig | undefined)
        : undefined;
      const status = axios.isAxiosError(error)
        ? error.response?.status
        : undefined;

      if (
        status === 401 &&
        flagged &&
        flagged.atlasAuth !== false &&
        !flagged.atlasRetried &&
        options.refreshSession
      ) {
        flagged.atlasRetried = true;
        const refreshed = await options.refreshSession();
        if (refreshed) {
          return instance.request(flagged);
        }
      }

      if (axios.isAxiosError(error) && error.response) {
        return normalizeResult(error.response.status, error.response.data);
      }

      return {
        status: 502,
        body: { success: false, message: UNREACHABLE_MESSAGE },
      } satisfies ApiResult<never>;
    },
  );

  return instance;
}

export async function atlasRequest<T>(
  instance: AxiosInstance,
  config: AtlasRequestConfig,
): Promise<ApiResult<T>> {
  try {
    const result: unknown = await instance.request(config);
    return result as ApiResult<T>;
  } catch {
    return {
      status: 502,
      body: { success: false, message: UNREACHABLE_MESSAGE },
    };
  }
}

function normalizeResult<T>(status: number, payload: unknown): ApiResult<T> {
  return {
    status,
    body: normalizeBody<T>(payload),
  };
}
