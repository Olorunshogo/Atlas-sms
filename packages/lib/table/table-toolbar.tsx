import type { ReactNode } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import { Search, X } from 'lucide-react';
import { cn } from '../utils';
import { fadeDownVariants, transitionBase } from '../motion/variants';
import type { BulkAction, TableSelectionApi } from './table-types';

/*
 * The toolbar: search, whatever filters the page owns, and the bulk bar.
 *
 * Search is debounced by the engine, not here - this is only the input.
 */

export interface TableToolbarProps<TRow> {
  showSearch: boolean;
  search: string;
  onSearchChange: (value: string) => void;
  searchPlaceholder: string;
  /** Filter chips, date pickers, anything the page owns. */
  children?: ReactNode;
  selection: TableSelectionApi<TRow>;
  bulkActions?: readonly BulkAction<TRow>[];
}

const bulkToneClasses = {
  default: 'border-line text-heading hover:bg-light-bg bg-white',
  primary: 'bg-primary text-white hover:opacity-90 border-primary',
  danger:
    'bg-status-cancelled text-status-cancelled-fg border-transparent hover:opacity-90',
} as const;

export function TableToolbar<TRow>({
  showSearch,
  search,
  onSearchChange,
  searchPlaceholder,
  children,
  selection,
  bulkActions,
}: TableToolbarProps<TRow>) {
  const hasBulk = selection.enabled && Boolean(bulkActions?.length);
  if (!showSearch && !children && !hasBulk) return null;

  return (
    <div className="border-line flex flex-col gap-3 border-b p-4">
      {(showSearch || children) && (
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
          {showSearch && (
            <div className="relative flex-1">
              <Search
                size={16}
                className="text-body pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 opacity-60"
              />
              <input
                type="search"
                value={search}
                onChange={(event) => onSearchChange(event.target.value)}
                placeholder={searchPlaceholder}
                aria-label={searchPlaceholder}
                className="border-line text-heading placeholder:text-placeholder-text focus:border-input-border-focus w-full rounded-lg border bg-white py-2.5 pr-9 pl-9 text-sm transition-all duration-300 ease-in-out outline-none"
              />
              {search && (
                <button
                  type="button"
                  aria-label="Clear search"
                  onClick={() => onSearchChange('')}
                  className="text-body absolute top-1/2 right-3 -translate-y-1/2 cursor-pointer rounded-full p-0.5 hover:bg-black/5"
                >
                  <X size={14} />
                </button>
              )}
            </div>
          )}

          {children && (
            <div className="flex items-center gap-2 overflow-x-auto">
              {children}
            </div>
          )}
        </div>
      )}

      <AnimatePresence>
        {hasBulk && selection.count > 0 && (
          <motion.div
            variants={fadeDownVariants}
            initial="initial"
            animate="animate"
            exit="exit"
            transition={transitionBase}
            className="bg-accent-soft flex flex-wrap items-center justify-between gap-3 rounded-xl px-4 py-2.5"
          >
            <span className="text-accent-soft-fg text-sm font-semibold">
              {selection.count} selected on this page
            </span>

            <div className="flex flex-wrap items-center gap-2">
              {bulkActions?.map((action) => {
                const Icon = action.icon;
                const disabled =
                  action.disabled?.(selection.selectedRows) ?? false;

                return (
                  <button
                    key={action.id}
                    type="button"
                    disabled={disabled}
                    onClick={() => void action.onSelect(selection.selectedRows)}
                    className={cn(
                      'inline-flex cursor-pointer items-center gap-1.5 rounded-full border px-4 py-1.5 text-sm font-semibold transition-all duration-300 ease-in-out disabled:cursor-not-allowed disabled:opacity-50',
                      bulkToneClasses[action.tone ?? 'default'],
                    )}
                  >
                    {Icon && <Icon size={14} />}
                    {action.label}
                  </button>
                );
              })}

              <button
                type="button"
                onClick={selection.clear}
                className="text-accent-soft-fg cursor-pointer text-sm font-semibold underline underline-offset-2"
              >
                Clear
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
