import { useState, type ReactNode } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import { ChevronDown } from 'lucide-react';
import { cn } from '../utils';
import {
  collapseVariants,
  staggerDelay,
  staggerItemVariants,
  transitionFast,
} from '../motion/variants';
import { densityTokens } from './table-density';
import { CardSkeletonList } from './table-skeleton';
import { TableRowActions } from './table-row-actions';
import type {
  RowAction,
  RowId,
  TableColumn,
  TableDensity,
  TableSelectionApi,
} from './table-types';

/*
 * The card renderer.
 *
 * The same column definitions as the table, arranged by each column's `priority`.
 * This is what makes a six-column admin table usable on a phone without a second, hand-maintained mobile markup tree - and it is also the list layout for pages that want cards at every width.
 */

function isColumnPriority<TRow>(
  column: TableColumn<TRow>,
  priority: TableColumn<TRow>['priority'],
): boolean {
  return (column.priority ?? 'secondary') === priority;
}

function labelFor<TRow>(column: TableColumn<TRow>): ReactNode {
  return column.cardLabel ?? column.header;
}

interface FieldRowsProps<TRow> {
  columns: readonly TableColumn<TRow>[];
  row: TRow;
}

function FieldRows<TRow>({ columns, row }: FieldRowsProps<TRow>) {
  return (
    <div className="flex flex-col gap-2">
      {columns.map((column) => (
        <div key={column.id} className="flex flex-col gap-0.5">
          <span className="text-body text-[0.6875rem] font-semibold tracking-wide uppercase opacity-70">
            {labelFor(column)}
          </span>
          <div className="text-heading text-sm">{column.cell(row)}</div>
        </div>
      ))}
    </div>
  );
}

export interface TableCardsProps<TRow> {
  columns: readonly TableColumn<TRow>[];
  rows: readonly TRow[];
  getRowId: (row: TRow) => RowId;
  density: TableDensity;
  selection: TableSelectionApi<TRow>;
  actions?: readonly RowAction<TRow>[];
  onRowClick?: (row: TRow) => void;
  loading: boolean;
  skeletonRowCount: number;
  emptyState: ReactNode;
  caption: string;
}

export function TableCards<TRow>({
  columns,
  rows,
  getRowId,
  density,
  selection,
  actions,
  onRowClick,
  loading,
  skeletonRowCount,
  emptyState,
  caption,
}: TableCardsProps<TRow>) {
  const tokens = densityTokens[density];
  const available = columns.filter((column) => !column.hideInCards);

  const primary = available.filter((c) => isColumnPriority(c, 'primary'));
  const trailing = available.filter((c) => isColumnPriority(c, 'trailing'));
  const secondary = available.filter((c) => isColumnPriority(c, 'secondary'));
  const detail = available.filter((c) => isColumnPriority(c, 'detail'));
  const inlineActionColumns = available.filter((c) =>
    isColumnPriority(c, 'actions'),
  );

  if (loading) {
    return <CardSkeletonList rowCount={skeletonRowCount} density={density} />;
  }

  if (rows.length === 0) {
    return <div className="py-12">{emptyState}</div>;
  }

  return (
    <ul aria-label={caption} className={cn('flex flex-col', tokens.cardGap)}>
      {rows.map((row, index) => (
        <TableCard
          key={getRowId(row)}
          row={row}
          index={index}
          density={density}
          primary={primary}
          trailing={trailing}
          secondary={secondary}
          detail={detail}
          inlineActionColumns={inlineActionColumns}
          selection={selection}
          actions={actions}
          onRowClick={onRowClick}
        />
      ))}
    </ul>
  );
}

interface TableCardProps<TRow> {
  row: TRow;
  index: number;
  density: TableDensity;
  primary: readonly TableColumn<TRow>[];
  trailing: readonly TableColumn<TRow>[];
  secondary: readonly TableColumn<TRow>[];
  detail: readonly TableColumn<TRow>[];
  inlineActionColumns: readonly TableColumn<TRow>[];
  selection: TableSelectionApi<TRow>;
  actions?: readonly RowAction<TRow>[];
  onRowClick?: (row: TRow) => void;
}

function TableCard<TRow>({
  row,
  index,
  density,
  primary,
  trailing,
  secondary,
  detail,
  inlineActionColumns,
  selection,
  actions,
  onRowClick,
}: TableCardProps<TRow>) {
  const [expanded, setExpanded] = useState<boolean>(false);
  const tokens = densityTokens[density];
  const clickable = Boolean(onRowClick);
  const hasFooter =
    (actions && actions.length > 0) || inlineActionColumns.length > 0;

  return (
    <motion.li
      variants={staggerItemVariants}
      initial="initial"
      animate="animate"
      transition={staggerDelay(index)}
      className={cn(
        'border-line flex flex-col overflow-hidden rounded-xl border bg-white',
        selection.isSelected(row) && 'border-primary',
      )}
    >
      <div
        role={clickable ? 'button' : undefined}
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
          'flex flex-col gap-3',
          tokens.cardPadding,
          clickable &&
            'focus-visible:outline-primary cursor-pointer focus-visible:outline-2 focus-visible:-outline-offset-2',
        )}
      >
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-start gap-3">
            {selection.enabled && (
              <input
                type="checkbox"
                aria-label="Select row"
                checked={selection.isSelected(row)}
                onClick={(event) => event.stopPropagation()}
                onChange={() => selection.toggleRow(row)}
                className="accent-primary mt-1 h-4 w-4 shrink-0 cursor-pointer"
              />
            )}
            <div className="flex min-w-0 flex-col gap-1">
              {primary.map((column) => (
                <div key={column.id} className="min-w-0">
                  {column.cell(row)}
                </div>
              ))}
            </div>
          </div>

          {trailing.length > 0 && (
            <div className="flex shrink-0 flex-col items-end gap-1.5">
              {trailing.map((column) => (
                <div key={column.id}>{column.cell(row)}</div>
              ))}
            </div>
          )}
        </div>

        {secondary.length > 0 && <FieldRows columns={secondary} row={row} />}

        {detail.length > 0 && (
          <div className="flex flex-col gap-2">
            <button
              type="button"
              aria-expanded={expanded}
              onClick={(event) => {
                event.stopPropagation();
                setExpanded((current) => !current);
              }}
              className="text-primary flex cursor-pointer items-center gap-1 self-start text-xs font-semibold"
            >
              {expanded ? 'Hide details' : 'More details'}
              <ChevronDown
                size={14}
                className={cn(
                  'transition-transform duration-200',
                  expanded && 'rotate-180',
                )}
              />
            </button>

            <AnimatePresence initial={false}>
              {expanded && (
                <motion.div
                  variants={collapseVariants}
                  initial="initial"
                  animate="animate"
                  exit="exit"
                  transition={transitionFast}
                  className="overflow-hidden"
                >
                  <FieldRows columns={detail} row={row} />
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        )}
      </div>

      {hasFooter && (
        <div
          className={cn(
            'border-line bg-light-bg/50 flex items-center justify-end gap-2 border-t px-4 py-2.5',
          )}
        >
          {inlineActionColumns.map((column) => (
            <div key={column.id}>{column.cell(row)}</div>
          ))}
          {actions && actions.length > 0 && (
            <TableRowActions row={row} actions={actions} layout="stacked" />
          )}
        </div>
      )}
    </motion.li>
  );
}
