import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { motion } from 'framer-motion';
import { ArrowDown, ArrowUp, ChevronsUpDown } from 'lucide-react';
import { cn } from '../utils';
import { staggerDelay, staggerItemVariants } from '../motion/variants';
import { alignClass, densityTokens } from './table-density';
import { TableSkeletonBody } from './table-skeleton';
import { TableRowActions } from './table-row-actions';
import type {
  RowAction,
  RowId,
  TableColumn,
  TableDensity,
  TableSelectionApi,
  TableSort,
} from './table-types';

/*
 * The desktop renderer: a real <table>.
 *
 * Real table semantics on purpose.
 * One of the lists this replaces was a CSS grid pretending to be a table, which meant no th, no scope, no row association, and nothing for a screen reader to announce.
 *
 * Horizontal overflow lives on an inner element rather than the card, so the card's rounded corners are never clipped mid-scroll, and the actions column is sticky so it cannot scroll out of reach on a narrow screen.
 */

interface ScrollShadows {
  left: boolean;
  right: boolean;
}

function useScrollShadows(): {
  ref: React.RefObject<HTMLDivElement | null>;
  shadows: ScrollShadows;
  onScroll: () => void;
} {
  const ref = useRef<HTMLDivElement>(null);
  const [shadows, setShadows] = useState<ScrollShadows>({
    left: false,
    right: false,
  });

  const measure = useCallback((): void => {
    const node = ref.current;
    if (!node) return;
    const maxScroll = node.scrollWidth - node.clientWidth;
    setShadows({
      left: node.scrollLeft > 1,
      /* 1px of slack: sub-pixel layout never lands exactly on the end. */
      right: maxScroll > 1 && node.scrollLeft < maxScroll - 1,
    });
  }, []);

  useEffect(() => {
    measure();
    const node = ref.current;
    if (!node || typeof ResizeObserver === 'undefined') return;

    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return () => observer.disconnect();
  }, [measure]);

  return { ref, shadows, onScroll: measure };
}

// === Header cell

interface HeaderCellProps<TRow> {
  column: TableColumn<TRow>;
  density: TableDensity;
  sort: TableSort | null;
  onToggleSort: (key: string) => void;
  sticky: boolean;
}

function HeaderCell<TRow>({
  column,
  density,
  sort,
  onToggleSort,
  sticky,
}: HeaderCellProps<TRow>) {
  const tokens = densityTokens[density];
  const key = column.sortKey ?? column.id;
  const active = sort?.key === key;
  const ariaSort = !column.sortable
    ? undefined
    : active
      ? sort.direction === 'asc'
        ? 'ascending'
        : 'descending'
      : 'none';

  const SortIcon = !active
    ? ChevronsUpDown
    : sort.direction === 'asc'
      ? ArrowUp
      : ArrowDown;

  return (
    <th
      scope="col"
      aria-sort={ariaSort}
      style={column.minWidth ? { minWidth: column.minWidth } : undefined}
      className={cn(
        'text-body bg-inherit text-xs font-semibold tracking-wide uppercase',
        tokens.headerCell,
        alignClass[column.align ?? 'left'],
        sticky && 'border-line sticky right-0 z-10 border-l',
      )}
    >
      {column.sortable ? (
        <button
          type="button"
          onClick={() => onToggleSort(key)}
          className={cn(
            'hover:text-heading inline-flex cursor-pointer items-center gap-1.5 transition-all duration-300 ease-in-out',
            active && 'text-heading',
          )}
        >
          <span className={cn(column.headerSrOnly && 'sr-only')}>
            {column.header}
          </span>
          <SortIcon size={13} className={cn(!active && 'opacity-40')} />
        </button>
      ) : (
        <span className={cn(column.headerSrOnly && 'sr-only')}>
          {column.header}
        </span>
      )}
    </th>
  );
}

// === Table

/*
 * `caption` is the screen-reader description of what the table lists.
 * `maxBodyHeight` turns the body into its own scroll area and makes the header sticky.
 * Without a height there is nothing for the header to stick inside: an overflow-x container has no vertical scroll of its own, so `sticky top-0` would silently do nothing against page scroll.
 */
export interface TableDesktopProps<TRow> {
  columns: readonly TableColumn<TRow>[];
  rows: readonly TRow[];
  getRowId: (row: TRow) => RowId;
  density: TableDensity;
  sort: TableSort | null;
  onToggleSort: (key: string) => void;
  selection: TableSelectionApi<TRow>;
  actions?: readonly RowAction<TRow>[];
  onRowClick?: (row: TRow) => void;
  caption: string;
  loading: boolean;
  skeletonRowCount: number;
  emptyState: ReactNode;
  maxBodyHeight?: string;
}

export function TableDesktop<TRow>({
  columns,
  rows,
  getRowId,
  density,
  sort,
  onToggleSort,
  selection,
  actions,
  onRowClick,
  caption,
  loading,
  skeletonRowCount,
  emptyState,
  maxBodyHeight,
}: TableDesktopProps<TRow>) {
  const tokens = densityTokens[density];
  const { ref, shadows, onScroll } = useScrollShadows();

  /* Rows arriving changes scrollWidth without resizing the container, so the ResizeObserver alone would leave the shadows stale. */
  useEffect(() => {
    onScroll();
  }, [rows.length, loading, onScroll]);

  const visibleColumns = columns.filter((column) => !column.hideInTable);
  const hasRowActions = Boolean(actions && actions.length > 0);
  const columnCount =
    visibleColumns.length +
    (selection.enabled ? 1 : 0) +
    (hasRowActions ? 1 : 0);

  return (
    <div className="relative">
      <div
        ref={ref}
        onScroll={onScroll}
        style={maxBodyHeight ? { maxHeight: maxBodyHeight } : undefined}
        className={cn('overflow-x-auto', maxBodyHeight && 'overflow-y-auto')}
      >
        <table aria-busy={loading} className="w-full border-collapse text-sm">
          <caption className="sr-only">{caption}</caption>

          <thead
            className={cn(
              'border-line border-b bg-white',
              maxBodyHeight && 'sticky top-0 z-20',
            )}
          >
            <tr className="bg-inherit">
              {selection.enabled && (
                <th
                  scope="col"
                  className={cn('w-10 bg-inherit', tokens.headerCell)}
                >
                  <input
                    type="checkbox"
                    aria-label="Select all rows on this page"
                    checked={selection.pageSelectionState === 'all'}
                    ref={(node) => {
                      if (node) {
                        node.indeterminate =
                          selection.pageSelectionState === 'some';
                      }
                    }}
                    onChange={selection.togglePage}
                    className="accent-primary h-4 w-4 cursor-pointer"
                  />
                </th>
              )}

              {visibleColumns.map((column) => (
                <HeaderCell
                  key={column.id}
                  column={column}
                  density={density}
                  sort={sort}
                  onToggleSort={onToggleSort}
                  sticky={!hasRowActions && column.priority === 'actions'}
                />
              ))}

              {hasRowActions && (
                <th
                  scope="col"
                  className={cn(
                    'text-body border-line sticky right-0 z-10 border-l bg-inherit text-right text-xs font-semibold tracking-wide uppercase',
                    tokens.headerCell,
                  )}
                >
                  Actions
                </th>
              )}
            </tr>
          </thead>

          {loading ? (
            <TableSkeletonBody
              columns={
                hasRowActions
                  ? [
                      ...visibleColumns,
                      { id: '__actions', header: '', cell: () => null },
                    ]
                  : visibleColumns
              }
              rowCount={skeletonRowCount}
              density={density}
              selectable={selection.enabled}
            />
          ) : rows.length === 0 ? (
            <tbody>
              <tr>
                <td colSpan={columnCount} className="px-5 py-12">
                  {emptyState}
                </td>
              </tr>
            </tbody>
          ) : (
            <tbody>
              {rows.map((row, index) => {
                const id = getRowId(row);
                const clickable = Boolean(onRowClick);

                return (
                  <motion.tr
                    key={id}
                    /* Enter only: an exit animation on a <tr> inside <tbody> is unreliable, and rows leave by being replaced. */
                    variants={staggerItemVariants}
                    initial="initial"
                    animate="animate"
                    transition={staggerDelay(index)}
                    tabIndex={clickable ? 0 : undefined}
                    onClick={clickable ? () => onRowClick?.(row) : undefined}
                    onKeyDown={
                      clickable
                        ? (event) => {
                            if (event.key === 'Enter' || event.key === ' ') {
                              event.preventDefault();
                              onRowClick?.(row);
                            }
                          }
                        : undefined
                    }
                    className={cn(
                      'border-line hover:bg-light-bg border-b bg-white transition-all duration-300 ease-in-out last:border-0',
                      clickable &&
                        'focus-visible:outline-primary cursor-pointer focus-visible:outline-2 focus-visible:-outline-offset-2',
                      selection.isSelected(row) && 'bg-accent-soft',
                    )}
                  >
                    {selection.enabled && (
                      <td
                        className={cn('bg-inherit', tokens.bodyCell)}
                        onClick={(event) => event.stopPropagation()}
                      >
                        <input
                          type="checkbox"
                          aria-label="Select row"
                          checked={selection.isSelected(row)}
                          onChange={() => selection.toggleRow(row)}
                          className="accent-primary h-4 w-4 cursor-pointer"
                        />
                      </td>
                    )}

                    {visibleColumns.map((column) => (
                      <td
                        key={column.id}
                        style={
                          column.minWidth
                            ? { minWidth: column.minWidth }
                            : undefined
                        }
                        className={cn(
                          'bg-inherit',
                          tokens.bodyCell,
                          alignClass[column.align ?? 'left'],
                          !hasRowActions &&
                            column.priority === 'actions' &&
                            'border-line sticky right-0 border-l',
                        )}
                      >
                        {column.cell(row)}
                      </td>
                    ))}

                    {hasRowActions && (
                      <td
                        className={cn(
                          'border-line sticky right-0 border-l bg-inherit',
                          tokens.bodyCell,
                        )}
                      >
                        <TableRowActions row={row} actions={actions ?? []} />
                      </td>
                    )}
                  </motion.tr>
                );
              })}
            </tbody>
          )}
        </table>
      </div>

      {/* Scroll affordances: without them the sticky actions column reads as the end of the table and the middle columns are never discovered. */}
      <div
        aria-hidden="true"
        className={cn(
          'pointer-events-none absolute inset-y-0 left-0 w-6 bg-linear-to-r from-black/8 to-transparent transition-opacity duration-200',
          shadows.left ? 'opacity-100' : 'opacity-0',
        )}
      />
      <div
        aria-hidden="true"
        className={cn(
          'pointer-events-none absolute inset-y-0 right-0 w-6 bg-linear-to-l from-black/8 to-transparent transition-opacity duration-200',
          shadows.right ? 'opacity-100' : 'opacity-0',
        )}
      />
    </div>
  );
}
