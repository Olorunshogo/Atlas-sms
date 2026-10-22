import type { ComponentType, ReactNode } from 'react';

/*
 * Types for the table engine.
 *
 * A consuming page declares a `columns` array describing its own rows, and the engine renders it two ways: a real <table> on desktop and a stack of cards on mobile, from the same definitions.
 * The engine knows no column names, the same way the dialog engine knows no dialog names.
 *
 * IMPORTANT: `columns` must be stable across renders.
 * Declare it at module scope, or wrap it in useMemo.
 * An inline array is a new identity every render, which re-derives every row on every keystroke.
 */

// === Layout

export type TableAlign = 'left' | 'center' | 'right';

/** Padding scale. One scale for every table, so density cannot drift. */
export type TableDensity = 'comfortable' | 'compact';

/** How the engine renders. `auto` is cards below md, table from md up. */
export type TableRenderMode = 'auto' | 'table' | 'cards';

/** Where the rows come from: sliced in the browser, or a page from the API. */
export type TableDataMode = 'client' | 'server';

/**
 * Where a column lands in card mode.
 *
 * - primary: the card headline, left of the top row. At most two read well.
 * - trailing: the right of the top row. Amounts and status pills.
 * - secondary: a label/value row in the card body, always visible.
 * - detail: collapsed behind the card's "More details" disclosure.
 * - actions: pinned to the card footer, and sticky-right in table mode.
 */
export type ColumnPriority =
  | 'primary'
  | 'trailing'
  | 'secondary'
  | 'detail'
  | 'actions';

// === Sorting

export type SortDirection = 'asc' | 'desc';

/** `key` is a column's `sortKey`, which is what the server allowlist expects. */
export interface TableSort {
  key: string;
  direction: SortDirection;
}

// === Columns

/*
 * `id` is the stable identity, also the default `sortKey`.
 * `minWidth` is any CSS width, applied as a min-width so content is never squeezed.
 * `priority` defaults to "secondary".
 * `headerSrOnly` hides the header text visually but keeps it for screen readers.
 * `sortKey` is the public sort key sent to the API, defaulting to `id`.
 * `sortValue` is the client-mode comparator source, required for sorting in client mode.
 * `searchValue` is the client-mode search source; a column without one is not searched.
 * `cardLabel` defaults to `header` when it is a string.
 * `hideInCards` and `hideInTable` skip this column entirely in the named mode.
 */
export interface TableColumn<TRow> {
  id: string;
  header: ReactNode;
  cell: (row: TRow) => ReactNode;

  align?: TableAlign;
  minWidth?: string;
  priority?: ColumnPriority;
  headerSrOnly?: boolean;

  sortable?: boolean;
  sortKey?: string;
  sortValue?: (row: TRow) => string | number | null | undefined;
  searchValue?: (row: TRow) => string;

  cardLabel?: ReactNode;
  hideInCards?: boolean;
  hideInTable?: boolean;
}

// === Row actions

/** `dialogKey` is a key in the app's dialog registry, opened via the dialog engine. */
export interface RowActionConfirm<TRow> {
  dialogKey: string;
  props?: (row: TRow) => Record<string, unknown>;
}

/*
 * `label` is also the accessible name when the action renders icon-only.
 * `iconOnly` shows the icon only in table mode; card mode always shows the label.
 * `isBusy` is per-row in-flight state, so one row's spinner cannot block the others.
 * `confirm` routes through the dialog engine before `onSelect` runs.
 */
export interface RowAction<TRow> {
  id: string;
  label: string;
  icon?: ComponentType<{ size?: number; className?: string }>;
  iconOnly?: boolean;
  onSelect?: (row: TRow) => void | Promise<void>;
  tone?: 'default' | 'primary' | 'danger';
  hidden?: (row: TRow) => boolean;
  disabled?: (row: TRow) => boolean;
  isBusy?: (row: TRow) => boolean;
  confirm?: RowActionConfirm<TRow>;
}

// === Bulk actions

export interface BulkAction<TRow> {
  id: string;
  label: string;
  icon?: ComponentType<{ size?: number; className?: string }>;
  tone?: 'default' | 'primary' | 'danger';
  onSelect: (rows: readonly TRow[]) => void | Promise<void>;
  disabled?: (rows: readonly TRow[]) => boolean;
}

// === State

export type RowId = string | number;

/*
 * What the engine emits to a server-mode consumer on every meaningful change.
 * `search` is already debounced.
 */
export interface TableStateSnapshot {
  page: number;
  pageSize: number;
  search: string;
  sort: TableSort | null;
}

export type PageSelectionState = 'none' | 'some' | 'all';

/*
 * `pageSelectionState`: select-all covers the visible page only.
 * In server mode the engine has never seen the other pages, so claiming to select them would be a lie.
 * `selectedRows` is the selected rows among those currently visible.
 */
export interface TableSelectionApi<TRow> {
  enabled: boolean;
  selectedIds: ReadonlySet<RowId>;
  count: number;
  isSelected: (row: TRow) => boolean;
  toggleRow: (row: TRow) => void;
  pageSelectionState: PageSelectionState;
  togglePage: () => void;
  clear: () => void;
  selectedRows: TRow[];
}

/*
 * `search` is the raw input value, updated on every keystroke; `debouncedSearch` is the value the engine actually filters and emits on.
 * `toggleSort` cycles asc, then desc, then off.
 * `visibleRows` is the rows to render: a client-side slice, or whatever the server returned.
 * `rangeStart` is the 1-based index of the first visible row, for "Showing 11 to 20 of 84".
 */
export interface TableStateApi<TRow> {
  page: number;
  pageSize: number;
  pageCount: number;
  total: number;
  setPage: (page: number) => void;
  setPageSize: (size: number) => void;

  search: string;
  setSearch: (value: string) => void;
  debouncedSearch: string;

  sort: TableSort | null;
  setSort: (sort: TableSort | null) => void;
  toggleSort: (key: string) => void;

  visibleRows: TRow[];
  getRowId: (row: TRow) => RowId;
  rangeStart: number;
  rangeEnd: number;

  selection: TableSelectionApi<TRow>;
  snapshot: TableStateSnapshot;
}
