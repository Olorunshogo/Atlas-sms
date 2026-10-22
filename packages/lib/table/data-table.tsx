import type { ReactNode } from 'react';
import { cn } from '../utils';
import { Pagination } from '../../components/pagination';
import {
  useMediaQuery,
  MD_BREAKPOINT_QUERY,
} from '../../hooks/use-media-query';
import { TableDesktop } from './table-desktop';
import { TableCards } from './table-cards';
import { TableToolbar } from './table-toolbar';
import { TableEmptyState, TableErrorState } from './table-states';
import { useTableState, type UseTableStateOptions } from './use-table-state';
import { SelectButtonField } from '../../components/select-button-field';
import type {
  BulkAction,
  RowAction,
  TableColumn,
  TableDensity,
  TableRenderMode,
  TableStateApi,
} from './table-types';

/*
 * The table engine.
 *
 * Solves, once, what every hand-rolled list in this codebase solved differently or not at all: pagination, sorting, debounced search, selection, per-row busy state, confirm dialogs, horizontal overflow, skeletons that match the columns, a real empty state that is not the error state, and a mobile layout that is not a six-column table on a phone.
 *
 * Never hand-roll a <table> in a page again.
 * Declare `columns` and render <DataTable>.
 *
 * `DataTable` owns its state.
 * Pages that need to read that state - a header that counts the selection, a filter bar wired to the same search - call `useTableState` themselves and render `DataTableView`.
 */

// === Presentation

/*
 * `caption` is the screen-reader name for the table, e.g. "Orders" - required, not optional.
 * `mode` defaults to "auto": cards below md, table from md up.
 * `error` is a message, not a boolean; it renders instead of rows, never as "no results".
 * `toolbar` is filter chips or anything else the page owns, placed beside the search.
 * `pageSizeOptions` adds a page-size selector next to the pager.
 * `maxBodyHeight` gives the table body its own scroll area and makes the header sticky.
 */
export interface DataTableViewProps<TRow> {
  state: TableStateApi<TRow>;
  columns: readonly TableColumn<TRow>[];

  caption: string;

  mode?: TableRenderMode;
  density?: TableDensity;

  actions?: readonly RowAction<TRow>[];
  bulkActions?: readonly BulkAction<TRow>[];
  onRowClick?: (row: TRow) => void;

  loading?: boolean;
  error?: string | null;
  onRetry?: () => void;

  emptyState?: ReactNode;

  showSearch?: boolean;
  searchPlaceholder?: string;
  toolbar?: ReactNode;

  showPagination?: boolean;
  pageSizeOptions?: readonly number[];

  maxBodyHeight?: string;

  className?: string;
}

export function DataTableView<TRow>({
  state,
  columns,
  caption,
  mode = 'auto',
  density = 'comfortable',
  actions,
  bulkActions,
  onRowClick,
  loading = false,
  error = null,
  onRetry,
  emptyState,
  showSearch = false,
  searchPlaceholder = 'Search',
  toolbar,
  showPagination = true,
  pageSizeOptions,
  maxBodyHeight,
  className,
}: DataTableViewProps<TRow>) {
  /*
   * One tree, not two hidden by CSS: a hidden duplicate would be read out by a screen reader alongside the visible one.
   * The server assumes the table, so SSR output is stable and the swap happens on hydration.
   */
  const isDesktopWidth = useMediaQuery(MD_BREAKPOINT_QUERY, true);
  const renderAsTable = mode === 'table' || (mode === 'auto' && isDesktopWidth);

  const skeletonRowCount = Math.min(state.pageSize, 6);

  /* An error replaces the rows entirely: a half-rendered table over a failed request is how "no results" ends up meaning "the API is down". */
  const body = error ? (
    <div className="px-5 py-12">
      <TableErrorState message={error} onRetry={onRetry} />
    </div>
  ) : renderAsTable ? (
    <TableDesktop
      columns={columns}
      rows={state.visibleRows}
      getRowId={state.getRowId}
      density={density}
      sort={state.sort}
      onToggleSort={state.toggleSort}
      selection={state.selection}
      actions={actions}
      onRowClick={onRowClick}
      caption={caption}
      loading={loading}
      skeletonRowCount={skeletonRowCount}
      emptyState={emptyState ?? <TableEmptyState />}
      maxBodyHeight={maxBodyHeight}
    />
  ) : (
    <div className="p-4">
      <TableCards
        columns={columns}
        rows={state.visibleRows}
        getRowId={state.getRowId}
        density={density}
        selection={state.selection}
        actions={actions}
        onRowClick={onRowClick}
        caption={caption}
        loading={loading}
        skeletonRowCount={skeletonRowCount}
        emptyState={emptyState ?? <TableEmptyState />}
      />
    </div>
  );

  const showFooter = showPagination && !error && !loading && state.total > 0;

  return (
    <div
      className={cn(
        'border-line overflow-hidden rounded-2xl border bg-white',
        className,
      )}
    >
      <TableToolbar
        showSearch={showSearch}
        search={state.search}
        onSearchChange={state.setSearch}
        searchPlaceholder={searchPlaceholder}
        selection={state.selection}
        bulkActions={bulkActions}
      >
        {toolbar}
      </TableToolbar>

      {body}

      {showFooter && (
        <div className="border-line flex flex-col items-center justify-between gap-3 border-t px-4 py-3 sm:flex-row">
          <p role="status" className="text-body text-xs">
            Showing {state.rangeStart} to {state.rangeEnd} of {state.total}
          </p>

          <div className="flex items-center gap-3">
            {pageSizeOptions && pageSizeOptions.length > 0 && (
              <SelectButtonField
                label="Per page"
                showInlineLabel
                size="sm"
                /* Sits at the right edge of the footer, so the menu opens leftward. */
                align="right"
                value={String(state.pageSize)}
                options={pageSizeOptions.map((size) => ({
                  value: String(size),
                  label: String(size),
                }))}
                onChange={(next) => state.setPageSize(Number(next))}
              />
            )}

            <Pagination
              currentPage={state.page}
              totalPages={state.pageCount}
              onPageChange={state.setPage}
            />
          </div>
        </div>
      )}
    </div>
  );
}

// === State-owning wrapper

export type DataTableProps<TRow> = Omit<DataTableViewProps<TRow>, 'state'> &
  Omit<UseTableStateOptions<TRow>, 'columns' | 'loading'>;

/**
 * The call site most pages want: hand it rows and columns, it does the rest.
 *
 * Reach for `useTableState` + `DataTableView` only when the page itself needs to read or drive the table's state.
 */
export function DataTable<TRow>(props: DataTableProps<TRow>) {
  const {
    rows,
    columns,
    dataMode,
    pageSize,
    initialPage,
    initialSearch,
    initialSort,
    getRowId,
    searchDebounceMs,
    selectable,
    total,
    pageCount,
    onStateChange,
    resetKey,
    ...viewProps
  } = props;

  const state = useTableState<TRow>({
    rows,
    columns,
    dataMode,
    pageSize,
    initialPage,
    initialSearch,
    initialSort,
    getRowId,
    searchDebounceMs,
    selectable,
    total,
    pageCount,
    onStateChange,
    resetKey,
    loading: viewProps.loading ?? false,
  });

  return <DataTableView state={state} columns={columns} {...viewProps} />;
}
