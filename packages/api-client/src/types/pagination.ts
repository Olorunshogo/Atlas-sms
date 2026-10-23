/*
 * Pagination contract shared by every list endpoint.
 *
 * The backend's response interceptor already carries `meta` alongside `data` for paginated handlers.
 * `apiFetch` unwraps to `data` and drops `meta`, which left the client with no total and no page count, so a paginated list could not render a pager.
 * These types plus `apiFetchPaged` are the other half.
 */

/*
 * `pages` is already computed server-side.
 * `sort` is echoed by endpoints that sort.
 * It reflects this rather than the request, so a rejected or defaulted sort shows the sort actually applied.
 */
export interface PaginationMeta {
  total: number;
  page: number;
  limit: number;
  pages: number;
  sort?: string;
  order?: SortDirection;
}

/*
 * `TMeta` is widened by endpoints that carry extras alongside the paging keys, a queue's sums, say, so a caller reads them without casting.
 */
export interface PagedResponse<
  T,
  TMeta extends PaginationMeta = PaginationMeta,
> {
  data: T[];
  meta: TMeta;
}

export type SortDirection = 'asc' | 'desc';

/**
 * Query params every paginated endpoint understands.
 * `sort` is a public key from the endpoint's own allowlist, never a column name - the server maps it.
 */
export interface PagedQuery {
  page?: number;
  limit?: number;
  sort?: string;
  order?: SortDirection;
  search?: string;
}

/** Widened so a caller can pass its own filters alongside the paging keys. */
export type QueryParams = Record<
  string,
  string | number | boolean | undefined | null
>;

/**
 * Serialises query params, dropping absent ones.
 * Empty strings are dropped too: `?search=` is a filter the caller did not ask for, and the backend's enum parsers treat it as absent anyway.
 */
export function buildQueryString(params: QueryParams = {}): string {
  const search = new URLSearchParams();

  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === null || value === '') continue;
    search.set(key, String(value));
  }

  const serialised = search.toString();
  return serialised ? `?${serialised}` : '';
}
