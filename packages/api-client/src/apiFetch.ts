import { BASE_BACKEND_URL } from './api';
import { getAccessToken, refreshTokens, clearTokens } from './auth';
import { decodeJwtPayload } from './auth-cookies';
import { announceRoleChanged } from './role-changed';
import {
  buildQueryString,
  type PagedResponse,
  type PaginationMeta,
  type QueryParams,
} from './types/pagination';

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    message: string,
    public readonly body?: unknown,
  ) {
    super(message);
  }
}

/*
 * The full response envelope the backend's interceptor emits.
 * `meta` is present only on paginated handlers.
 */
interface ApiEnvelope<T> {
  data: T;
  meta?: PaginationMeta;
  message?: string;
}

/**
 * Shared admin header key for sub-admin browser calls.
 * Must match SUPER_ADMIN_KEY_1 or SUPER_ADMIN_KEY_2 on the backend.
 */
function sharedAdminKey(): string | undefined {
  const fromVite = import.meta.env['VITE_ADMIN_SHARED_KEY'] as
    | string
    | undefined;
  return fromVite?.trim() || undefined;
}

function appendAdminHeaders(
  headers: Record<string, string>,
  token: string | null,
): void {
  if (!token) return;
  const payload = decodeJwtPayload<{
    role?: string;
    admin_tier?: string;
  }>(token);
  if (payload?.role !== 'admin' || payload.admin_tier !== 'sub') return;

  const key = sharedAdminKey();
  if (!key) return;

  headers['X-Admin-Key'] = key;
  headers['X-Admin-Tier'] = 'sub';
}

async function readRoleChange(res: Response): Promise<{
  role: string | undefined;
  message: string;
  body: unknown;
} | null> {
  try {
    const body = (await res.clone().json()) as {
      code?: string;
      role?: string;
      message?: string;
    };
    if (body.code !== 'ROLE_CHANGED') return null;
    return {
      role: body.role,
      message: body.message ?? 'Your role has changed. Please sign in again.',
      body,
    };
  } catch {
    return null;
  }
}

/*
 * One request, one silent refresh on 401, one retry.
 * Both apiFetch and apiFetchPaged go through here so the auth behaviour cannot drift apart.
 */
async function requestEnvelope<T>(
  path: string,
  options: RequestInit,
): Promise<ApiEnvelope<T>> {
  const token = getAccessToken();
  /* FormData sets its own multipart Content-Type, boundary included, so a JSON one here would break parsing. */
  const isFormData: boolean =
    typeof FormData !== 'undefined' && options.body instanceof FormData;
  const headers: Record<string, string> = {
    ...(isFormData ? {} : { 'Content-Type': 'application/json' }),
    ...(options.headers as Record<string, string>),
  };
  if (token) headers['Authorization'] = `Bearer ${token}`;
  appendAdminHeaders(headers, token);

  const url = `${BASE_BACKEND_URL}${path}`;
  let res = await fetch(url, { ...options, headers, credentials: 'include' });

  if (res.status === 401) {
    // Checked before the refresh: the server cleared this session's refresh token on purpose, so retrying would only turn the news into "session expired".
    const roleChange = await readRoleChange(res);
    if (roleChange) {
      clearTokens();
      announceRoleChanged(roleChange.role);
      throw new ApiError(401, roleChange.message, roleChange.body);
    }
    try {
      const { accessToken } = await refreshTokens();
      headers['Authorization'] = `Bearer ${accessToken}`;
      appendAdminHeaders(headers, accessToken);
      res = await fetch(url, { ...options, headers, credentials: 'include' });
    } catch {
      clearTokens();
      throw new ApiError(401, 'Session expired. Please log in again.');
    }
  }

  if (!res.ok) {
    let body: unknown;
    try {
      body = await res.json();
    } catch {
      body = null;
    }
    const msg =
      (body as { message?: string })?.message ??
      `Request failed: ${res.status}`;
    throw new ApiError(res.status, msg, body);
  }

  return (await res.json()) as ApiEnvelope<T>;
}

/**
 * Authenticated fetch with automatic token refresh.
 * Throws ApiError on non-2xx responses.
 * On 401, attempts one silent refresh then retries.
 * On second 401, clears tokens and throws - caller should redirect to /login.
 */
export async function apiFetch<T = unknown>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const envelope = await requestEnvelope<T>(path, options);
  return envelope.data;
}

/**
 * Authenticated fetch for a paginated list endpoint, keeping `meta`.
 *
 * Use this anywhere a pager, a total, or server-side sorting is involved.
 * `params` is serialised onto the path, so callers never hand-build a query string: apiFetchPaged("/admin/orders", { page, limit, sort, order, search }).
 */
export async function apiFetchPaged<
  T = unknown,
  TMeta extends PaginationMeta = PaginationMeta,
>(
  path: string,
  params: QueryParams = {},
  options: RequestInit = {},
): Promise<PagedResponse<T, TMeta>> {
  const envelope = await requestEnvelope<T[]>(
    `${path}${buildQueryString(params)}`,
    options,
  );

  /*
   * A missing `meta` means the endpoint is not actually paginated, so the caller is wrong.
   * Failing loudly beats rendering a pager over a full list that silently shows only its first page.
   */
  if (!envelope.meta) {
    throw new ApiError(
      500,
      `${path} returned no pagination meta. Use apiFetch, or make the endpoint paginated.`,
    );
  }

  return { data: envelope.data ?? [], meta: envelope.meta as TMeta };
}

/**
 * POST/PUT/DELETE mutation with automatic token refresh.
 * Convenience wrapper around apiFetch for mutation operations.
 */
export async function apiMutate<T = unknown>(
  path: string,
  options: RequestInit & { method: 'POST' | 'PUT' | 'DELETE' | 'PATCH' },
): Promise<T> {
  return apiFetch<T>(path, options);
}
