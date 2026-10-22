import {
  computed,
  ref,
  toValue,
  watch,
  type ComputedRef,
  type MaybeRefOrGetter,
  type Ref,
} from 'vue';
import type {
  RowId,
  TableColumn,
  TableSort,
  TableStateSnapshot,
} from './table-types';

export interface UseTableStateOptions<TRow> {
  rows: MaybeRefOrGetter<readonly TRow[]>;
  columns: readonly TableColumn<TRow>[];
  dataMode?: 'client' | 'server';
  pageSize?: number;
  initialPage?: number;
  initialSearch?: string;
  initialSort?: TableSort | null;
  getRowId?: (row: TRow) => RowId;
  searchDebounceMs?: number;
  loading?: MaybeRefOrGetter<boolean>;
  total?: MaybeRefOrGetter<number | undefined>;
  pageCount?: MaybeRefOrGetter<number | undefined>;
  resetKey?: MaybeRefOrGetter<string | number | boolean | null>;
  onStateChange?: (state: TableStateSnapshot) => void;
}

export interface TableStateApi<TRow> {
  page: Ref<number>;
  pageSize: Ref<number>;
  pageCount: ComputedRef<number>;
  total: ComputedRef<number>;
  search: Ref<string>;
  debouncedSearch: Ref<string>;
  sort: Ref<TableSort | null>;
  visibleRows: ComputedRef<TRow[]>;
  rangeStart: ComputedRef<number>;
  rangeEnd: ComputedRef<number>;
  setPage: (next: number) => void;
  setSearch: (next: string) => void;
  toggleSort: (key: string) => void;
  getRowId: (row: TRow) => RowId;
  selectedIds: Ref<ReadonlySet<RowId>>;
  pageSelection: ComputedRef<'none' | 'some' | 'all'>;
  selectedRows: ComputedRef<TRow[]>;
  toggleRow: (row: TRow) => void;
  togglePage: () => void;
  clearSelection: () => void;
}

function defaultGetRowId<TRow>(row: TRow): RowId {
  const id = (row as { id?: RowId }).id;
  if (id === undefined || id === null) {
    throw new Error('[useTableState] Rows have no id. Pass getRowId.');
  }
  return id;
}

function compareValues(
  left: string | number | null | undefined,
  right: string | number | null | undefined,
): number {
  const leftMissing = left === null || left === undefined;
  const rightMissing = right === null || right === undefined;
  if (leftMissing && rightMissing) {
    return 0;
  }
  if (leftMissing) {
    return 1;
  }
  if (rightMissing) {
    return -1;
  }
  if (typeof left === 'number' && typeof right === 'number') {
    return left - right;
  }
  return String(left).localeCompare(String(right), undefined, {
    numeric: true,
  });
}

export function useTableState<TRow>(
  options: UseTableStateOptions<TRow>,
): TableStateApi<TRow> {
  const dataMode = options.dataMode ?? 'client';
  const getRowId = options.getRowId ?? defaultGetRowId;
  const searchDebounceMs = options.searchDebounceMs ?? 300;
  const page = ref<number>(options.initialPage ?? 1);
  const pageSize = ref<number>(options.pageSize ?? 10);
  const search = ref<string>(options.initialSearch ?? '');
  const debouncedSearch = ref<string>(options.initialSearch ?? '');
  const sort = ref<TableSort | null>(options.initialSort ?? null);
  const selectedIds = ref<ReadonlySet<RowId>>(new Set());
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  watch(search, (value: string) => {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }
    debounceTimer = setTimeout(() => {
      debouncedSearch.value = value;
    }, searchDebounceMs);
  });

  const filteredRows = computed((): readonly TRow[] => {
    const rows = toValue(options.rows);
    if (dataMode === 'server') {
      return rows;
    }
    const query = debouncedSearch.value.trim().toLowerCase();
    const searchable = options.columns.filter((column) => column.searchValue);
    if (!query || searchable.length === 0) {
      return rows;
    }
    return rows.filter((row) => {
      return searchable.some((column) => {
        const value = column.searchValue ? column.searchValue(row) : '';
        return value.toLowerCase().includes(query);
      });
    });
  });

  const sortedRows = computed((): readonly TRow[] => {
    if (dataMode === 'server' || !sort.value) {
      return filteredRows.value;
    }
    const current = sort.value;
    const column = options.columns.find((candidate) => {
      return (candidate.sortKey ?? candidate.id) === current.key;
    });
    if (!column?.sortValue) {
      return filteredRows.value;
    }
    const read = column.sortValue;
    const factor = current.direction === 'asc' ? 1 : -1;
    return [...filteredRows.value].sort((left, right) => {
      return compareValues(read(left), read(right)) * factor;
    });
  });

  const total = computed((): number => {
    if (dataMode === 'server') {
      return toValue(options.total) ?? toValue(options.rows).length;
    }
    return sortedRows.value.length;
  });

  const pageCount = computed((): number => {
    if (dataMode === 'server') {
      const provided = toValue(options.pageCount);
      if (provided !== undefined) {
        return Math.max(1, provided);
      }
    }
    return Math.max(1, Math.ceil(total.value / pageSize.value));
  });

  const visibleRows = computed((): TRow[] => {
    if (dataMode === 'server') {
      return [...toValue(options.rows)];
    }
    const start = (page.value - 1) * pageSize.value;
    return sortedRows.value.slice(start, start + pageSize.value);
  });

  const rangeStart = computed((): number => {
    if (total.value === 0) {
      return 0;
    }
    return (page.value - 1) * pageSize.value + 1;
  });

  const rangeEnd = computed((): number => {
    return Math.min(page.value * pageSize.value, total.value);
  });

  watch(
    [debouncedSearch, sort, pageSize, () => toValue(options.resetKey)],
    () => {
      page.value = 1;
    },
  );

  watch([debouncedSearch, () => toValue(options.resetKey)], () => {
    selectedIds.value = new Set();
  });

  watch([page, pageCount, () => toValue(options.loading)], () => {
    if (toValue(options.loading)) {
      return;
    }
    if (page.value > pageCount.value) {
      page.value = pageCount.value;
    }
  });

  watch(
    [
      page,
      pageSize,
      debouncedSearch,
      sort,
      () => String(toValue(options.resetKey) ?? ''),
    ],
    () => {
      if (!options.onStateChange) {
        return;
      }
      options.onStateChange({
        page: page.value,
        pageSize: pageSize.value,
        search: debouncedSearch.value,
        sort: sort.value,
      });
    },
  );

  const pageIds = computed((): RowId[] => {
    return visibleRows.value.map((row) => getRowId(row));
  });

  const pageSelection = computed((): 'none' | 'some' | 'all' => {
    if (pageIds.value.length === 0) {
      return 'none';
    }
    const selectedOnPage = pageIds.value.filter((id) =>
      selectedIds.value.has(id),
    ).length;
    if (selectedOnPage === 0) {
      return 'none';
    }
    if (selectedOnPage === pageIds.value.length) {
      return 'all';
    }
    return 'some';
  });

  const selectedRows = computed((): TRow[] => {
    return toValue(options.rows).filter((row) =>
      selectedIds.value.has(getRowId(row)),
    );
  });

  function toggleRow(row: TRow): void {
    const id = getRowId(row);
    const next = new Set(selectedIds.value);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selectedIds.value = next;
  }

  function togglePage(): void {
    const next = new Set(selectedIds.value);
    const allSelected = pageIds.value.every((id) => next.has(id));
    for (const id of pageIds.value) {
      if (allSelected) {
        next.delete(id);
      } else {
        next.add(id);
      }
    }
    selectedIds.value = next;
  }

  function clearSelection(): void {
    selectedIds.value = new Set();
  }

  function setPage(next: number): void {
    page.value = next;
  }

  function setSearch(next: string): void {
    search.value = next;
  }

  function toggleSort(key: string): void {
    const current = sort.value;
    if (!current || current.key !== key) {
      sort.value = { key, direction: 'asc' };
      return;
    }
    if (current.direction === 'asc') {
      sort.value = { key, direction: 'desc' };
      return;
    }
    sort.value = null;
  }

  return {
    page,
    pageSize,
    pageCount,
    total,
    search,
    debouncedSearch,
    sort,
    visibleRows,
    rangeStart,
    rangeEnd,
    setPage,
    setSearch,
    toggleSort,
    getRowId,
    selectedIds,
    pageSelection,
    selectedRows,
    toggleRow,
    togglePage,
    clearSelection,
  };
}
