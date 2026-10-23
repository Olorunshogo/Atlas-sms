import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useDebouncedValue } from '../../hooks/use-debounced-value';
import type {
  PageSelectionState,
  RowId,
  TableColumn,
  TableSelectionApi,
  TableSort,
  TableStateApi,
  TableStateSnapshot,
} from './table-types';

/*
 * The table engine's brain. No DOM, no markup - so the same pagination, sort, search, selection and page-reset behaviour is shared by the table renderer, the card renderer, and anything else that ever renders rows.
 */

/*
 * `rows` is the whole set in client mode, the current page in server mode.
 * `dataMode` defaults to "client"; `pageSize` defaults to 10.
 * `getRowId` is required when rows carry no `id`.
 * `loading` is whether the consumer is currently fetching.
 * The engine only clamps an out-of-range page while this is false: a server-mode consumer that empties its rows during a fetch would otherwise be yanked back to page 1 by the very request it just made.
 * `total` and `pageCount` are server mode only.
 * `onStateChange` is called with the debounced state whenever it changes, and once on mount so the consumer can make its first request.
 * It is held in a ref, so an inline arrow function here does not cause a refetch loop.
 * `resetKey` is the identity of any filter the page owns outside the engine, such as a status chip row.
 * When it changes, the engine returns to page 1 and clears the selection, which is the reset every hand-rolled list forgets.
 */
export interface UseTableStateOptions<TRow> {
  rows: readonly TRow[];
  columns: readonly TableColumn<TRow>[];
  dataMode?: TableDataModeOption;
  pageSize?: number;
  initialPage?: number;
  initialSearch?: string;
  initialSort?: TableSort | null;
  getRowId?: (row: TRow) => RowId;
  searchDebounceMs?: number;
  selectable?: boolean;
  loading?: boolean;

  total?: number;
  pageCount?: number;
  onStateChange?: (state: TableStateSnapshot) => void;

  resetKey?: string | number | boolean | null;
}

type TableDataModeOption = 'client' | 'server';

const EMPTY_SELECTION: ReadonlySet<RowId> = new Set<RowId>();

function defaultGetRowId<TRow>(row: TRow): RowId {
  const id = (row as { id?: RowId }).id;
  if (id === undefined || id === null) {
    /* A wiring mistake, not a user-facing error - fail loudly. */
    throw new Error(
      '[useTableState] Rows have no `id`. Pass getRowId to identify them.',
    );
  }
  return id;
}

/* Nulls sort last in both directions: absent is not smaller, it is absent. */
function compareValues(
  a: string | number | null | undefined,
  b: string | number | null | undefined,
): number {
  const aMissing = a === null || a === undefined;
  const bMissing = b === null || b === undefined;
  if (aMissing && bMissing) return 0;
  if (aMissing) return 1;
  if (bMissing) return -1;

  if (typeof a === 'number' && typeof b === 'number') return a - b;
  return String(a).localeCompare(String(b), undefined, { numeric: true });
}

export function useTableState<TRow>(
  options: UseTableStateOptions<TRow>,
): TableStateApi<TRow> {
  const {
    rows,
    columns,
    dataMode = 'client',
    pageSize: initialPageSize = 10,
    initialPage = 1,
    initialSearch = '',
    initialSort = null,
    getRowId = defaultGetRowId,
    searchDebounceMs = 300,
    selectable = false,
    total: serverTotal,
    pageCount: serverPageCount,
    onStateChange,
    resetKey = null,
    loading = false,
  } = options;

  const [page, setPage] = useState<number>(initialPage);
  const [pageSize, setPageSize] = useState<number>(initialPageSize);
  const [search, setSearch] = useState<string>(initialSearch);
  const [sort, setSort] = useState<TableSort | null>(initialSort);
  const [selectedIds, setSelectedIds] =
    useState<ReadonlySet<RowId>>(EMPTY_SELECTION);

  const debouncedSearch = useDebouncedValue<string>(search, searchDebounceMs);
  const isServer = dataMode === 'server';

  // === Client-side derivation

  const searchable = useMemo(
    () => columns.filter((column) => Boolean(column.searchValue)),
    [columns],
  );

  const filteredRows = useMemo<readonly TRow[]>(() => {
    if (isServer) return rows;
    const query = debouncedSearch.trim().toLowerCase();
    if (!query || searchable.length === 0) return rows;

    return rows.filter((row) =>
      searchable.some((column) =>
        (column.searchValue?.(row) ?? '').toLowerCase().includes(query),
      ),
    );
  }, [isServer, rows, debouncedSearch, searchable]);

  const sortedRows = useMemo<readonly TRow[]>(() => {
    if (isServer || !sort) return filteredRows;

    const column = columns.find(
      (candidate) => (candidate.sortKey ?? candidate.id) === sort.key,
    );
    if (!column?.sortValue) return filteredRows;

    const read = column.sortValue;
    const factor = sort.direction === 'asc' ? 1 : -1;
    /* Copy first: Array.prototype.sort mutates, and `rows` is the caller's. */
    return [...filteredRows].sort(
      (a, b) => compareValues(read(a), read(b)) * factor,
    );
  }, [isServer, filteredRows, sort, columns]);

  const total = isServer ? (serverTotal ?? rows.length) : sortedRows.length;

  const pageCount = isServer
    ? (serverPageCount ?? Math.max(1, Math.ceil(total / pageSize)))
    : Math.max(1, Math.ceil(total / pageSize));

  // === Resets, derived during render

  /*
   * Adjusting state during render rather than in an effect, so the change is committed in the same pass.
   * In an effect, the emission below would fire once with the stale page and again with the reset one - two requests for one keystroke.
   */
  const pageResetSignature = `${debouncedSearch}|${sort?.key ?? ''}|${
    sort?.direction ?? ''
  }|${pageSize}|${String(resetKey)}`;
  const [lastPageReset, setLastPageReset] =
    useState<string>(pageResetSignature);
  if (lastPageReset !== pageResetSignature) {
    setLastPageReset(pageResetSignature);
    if (page !== 1) setPage(1);
  }

  /* Sorting reorders rows a user already chose; a new filter set does not. */
  const selectionResetSignature = `${debouncedSearch}|${String(resetKey)}`;
  const [lastSelectionReset, setLastSelectionReset] = useState<string>(
    selectionResetSignature,
  );
  if (lastSelectionReset !== selectionResetSignature) {
    setLastSelectionReset(selectionResetSignature);
    if (selectedIds.size > 0) setSelectedIds(EMPTY_SELECTION);
  }

  /* A shrinking result set can strand the viewer on a page that no longer exists. */
  if (!loading && page > pageCount) setPage(pageCount);

  const safePage = Math.min(page, pageCount);

  const visibleRows = useMemo<TRow[]>(() => {
    if (isServer) return [...rows];
    const start = (safePage - 1) * pageSize;
    return sortedRows.slice(start, start + pageSize);
  }, [isServer, rows, sortedRows, safePage, pageSize]);

  // === Emission

  const snapshot = useMemo<TableStateSnapshot>(
    () => ({
      page: safePage,
      pageSize,
      search: debouncedSearch,
      sort,
    }),
    [safePage, pageSize, debouncedSearch, sort],
  );

  const onStateChangeRef = useRef(onStateChange);
  onStateChangeRef.current = onStateChange;

  /*
   * resetKey belongs in this key, not just in the page-reset signature above.
   *
   * A page-level filter changes resetKey but usually leaves page/search/sort untouched - the admin is already on page 1.
   * Without resetKey here the serialised snapshot is identical, the effect below never re-runs, and a server-mode table never reloads for the new filter.
   */
  const snapshotKey = `${snapshot.page}|${snapshot.pageSize}|${
    snapshot.search
  }|${snapshot.sort?.key ?? ''}|${snapshot.sort?.direction ?? ''}|${String(
    resetKey,
  )}`;

  useEffect(() => {
    onStateChangeRef.current?.(snapshot);
    /* Keyed on the serialised snapshot: object identity would fire every render. */
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [snapshotKey]);

  // === Sorting controls

  const toggleSort = useCallback((key: string): void => {
    setSort((current) => {
      if (current?.key !== key) return { key, direction: 'asc' };
      if (current.direction === 'asc') return { key, direction: 'desc' };
      return null;
    });
  }, []);

  // === Selection

  const isSelected = useCallback(
    (row: TRow): boolean => selectedIds.has(getRowId(row)),
    [selectedIds, getRowId],
  );

  const toggleRow = useCallback(
    (row: TRow): void => {
      const id = getRowId(row);
      setSelectedIds((current) => {
        const next = new Set(current);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        return next;
      });
    },
    [getRowId],
  );

  const pageIds = useMemo<RowId[]>(
    () => visibleRows.map(getRowId),
    [visibleRows, getRowId],
  );

  const pageSelectionState = useMemo<PageSelectionState>(() => {
    if (pageIds.length === 0) return 'none';
    const selectedOnPage = pageIds.filter((id) => selectedIds.has(id)).length;
    if (selectedOnPage === 0) return 'none';
    return selectedOnPage === pageIds.length ? 'all' : 'some';
  }, [pageIds, selectedIds]);

  const togglePage = useCallback((): void => {
    setSelectedIds((current) => {
      const next = new Set(current);
      const allSelected = pageIds.every((id) => next.has(id));
      for (const id of pageIds) {
        if (allSelected) next.delete(id);
        else next.add(id);
      }
      return next;
    });
  }, [pageIds]);

  const clearSelection = useCallback((): void => {
    setSelectedIds(EMPTY_SELECTION);
  }, []);

  const selectedRows = useMemo<TRow[]>(
    () => visibleRows.filter((row) => selectedIds.has(getRowId(row))),
    [visibleRows, selectedIds, getRowId],
  );

  const selection = useMemo<TableSelectionApi<TRow>>(
    () => ({
      enabled: selectable,
      selectedIds,
      count: selectedIds.size,
      isSelected,
      toggleRow,
      pageSelectionState,
      togglePage,
      clear: clearSelection,
      selectedRows,
    }),
    [
      selectable,
      selectedIds,
      isSelected,
      toggleRow,
      pageSelectionState,
      togglePage,
      clearSelection,
      selectedRows,
    ],
  );

  const rangeStart = total === 0 ? 0 : (safePage - 1) * pageSize + 1;
  const rangeEnd = Math.min(safePage * pageSize, total);

  return {
    page: safePage,
    pageSize,
    pageCount,
    total,
    setPage,
    setPageSize,
    search,
    setSearch,
    debouncedSearch,
    sort,
    setSort,
    toggleSort,
    visibleRows,
    getRowId,
    rangeStart,
    rangeEnd,
    selection,
    snapshot,
  };
}
