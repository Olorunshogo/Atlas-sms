export interface PaginationMeta {
  total: number;
  page: number;
  limit: number;
  pages: number;
  sort?: string;
  order?: SortDirection;
}
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
export declare function buildQueryString(params?: QueryParams): string;
