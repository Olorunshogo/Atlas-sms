import {
  type PagedResponse,
  type PaginationMeta,
  type QueryParams,
} from './types/pagination';
export declare class ApiError extends Error {
  readonly status: number;
  readonly body?: unknown | undefined;
  constructor(status: number, message: string, body?: unknown | undefined);
}
/**
 * Authenticated fetch with automatic token refresh.
 * Throws ApiError on non-2xx responses.
 * On 401, attempts one silent refresh then retries.
 * On second 401, clears tokens and throws - caller should redirect to /login.
 */
export declare function apiFetch<T = unknown>(
  path: string,
  options?: RequestInit,
): Promise<T>;
/**
 * Authenticated fetch for a paginated list endpoint, keeping `meta`.
 *
 * Use this anywhere a pager, a total, or server-side sorting is involved.
 * `params` is serialised onto the path, so callers never hand-build a query string: apiFetchPaged("/admin/orders", { page, limit, sort, order, search }).
 */
export declare function apiFetchPaged<
  T = unknown,
  TMeta extends PaginationMeta = PaginationMeta,
>(
  path: string,
  params?: QueryParams,
  options?: RequestInit,
): Promise<PagedResponse<T, TMeta>>;
/**
 * POST/PUT/DELETE mutation with automatic token refresh.
 * Convenience wrapper around apiFetch for mutation operations.
 */
export declare function apiMutate<T = unknown>(
  path: string,
  options: RequestInit & {
    method: 'POST' | 'PUT' | 'DELETE' | 'PATCH';
  },
): Promise<T>;
