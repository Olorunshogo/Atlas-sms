/*
 * A page declares columns.
 * The engine renders a table from lg up and cards below that, from the same definitions.
 * columns must stay stable: declare them once in setup, not inside a computed that rebuilds every keystroke.
 */

export type TableAlign = 'left' | 'center' | 'right';

export type SortDirection = 'asc' | 'desc';

export type RowId = string | number;

export interface TableSort {
  key: string;
  direction: SortDirection;
}

export interface TableColumn<TRow> {
  id: string;
  header: string;
  cell: (row: TRow) => string;
  align?: TableAlign;
  sortable?: boolean;
  sortKey?: string;
  sortValue?: (row: TRow) => string | number | null | undefined;
  searchValue?: (row: TRow) => string;
}

export interface RowActionConfirm<TRow> {
  dialogKey: string;
  props?: (row: TRow) => Record<string, unknown>;
}

export interface BulkAction<TRow> {
  id: string;
  label: string;
  tone?: 'default' | 'primary' | 'danger';
  onSelect: (rows: readonly TRow[]) => void | Promise<void>;
}

export interface RowAction<TRow> {
  id: string;
  label: string;
  onSelect?: (row: TRow) => void | Promise<void>;
  tone?: 'default' | 'primary' | 'danger';
  hidden?: (row: TRow) => boolean;
  disabled?: (row: TRow) => boolean;
  confirm?: RowActionConfirm<TRow>;
}

export interface TableStateSnapshot {
  page: number;
  pageSize: number;
  search: string;
  sort: TableSort | null;
}
