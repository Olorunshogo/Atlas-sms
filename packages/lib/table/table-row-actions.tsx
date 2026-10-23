import { Loader2 } from 'lucide-react';
import { cn } from '../utils';
import { useOptionalDialog } from '../dialog/dialog-provider';
import type { RowAction } from './table-types';

/*
 * Row actions.
 *
 * Every list in this codebase had grown its own per-row busy flag - deletingId, busyId, actioningId - and its own confirmation, or none at all.
 * This is the one implementation: `isBusy(row)` for the spinner, and `confirm` to route through the dialog engine first.
 *
 * A confirm dialog is handed `onConfirm` alongside its own props, so the dialog decides when the action runs and the engine never assumes the answer.
 */

const toneClasses = {
  default: 'text-body hover:bg-black/5',
  primary: 'text-primary hover:bg-primary/10',
  danger: 'text-status-cancelled-fg hover:bg-status-cancelled',
} as const;

export interface TableRowActionsProps<TRow> {
  row: TRow;
  actions: readonly RowAction<TRow>[];
  /** Icon buttons in a table cell, labelled buttons in a card footer. */
  layout?: 'inline' | 'stacked';
}

export function TableRowActions<TRow>({
  row,
  actions,
  layout = 'inline',
}: TableRowActionsProps<TRow>) {
  const dialog = useOptionalDialog();

  const visible = actions.filter((action) => !action.hidden?.(row));
  if (visible.length === 0) return null;

  function run(action: RowAction<TRow>): void {
    if (action.confirm) {
      if (!dialog) {
        console.error(
          `[DataTable] Row action "${action.id}" declares a confirm dialog, but there is no <DialogProvider> above this table.`,
        );
        return;
      }
      dialog.triggerDialog(action.confirm.dialogKey, {
        ...action.confirm.props?.(row),
        onConfirm: () => action.onSelect?.(row),
      });
      return;
    }

    void action.onSelect?.(row);
  }

  return (
    <div
      className={cn(
        'flex items-center gap-2',
        layout === 'stacked' ? 'flex-wrap' : 'justify-end',
      )}
    >
      {visible.map((action) => {
        const busy = action.isBusy?.(row) ?? false;
        const disabled = busy || (action.disabled?.(row) ?? false);
        const Icon = action.icon;
        /* A card footer always shows words: an icon alone is a guess there. */
        const showLabel = layout === 'stacked' || !action.iconOnly;

        return (
          <button
            key={action.id}
            type="button"
            title={action.label}
            aria-label={showLabel ? undefined : action.label}
            aria-busy={busy}
            disabled={disabled}
            onClick={(event) => {
              /* The row itself may be clickable; this click is not for it. */
              event.stopPropagation();
              run(action);
            }}
            className={cn(
              'inline-flex cursor-pointer items-center gap-1.5 rounded-lg text-sm font-medium transition-all duration-300 ease-in-out',
              showLabel ? 'px-3 py-1.5' : 'p-1.5',
              toneClasses[action.tone ?? 'default'],
              'disabled:cursor-not-allowed disabled:opacity-50',
            )}
          >
            {busy ? (
              <Loader2 size={15} className="animate-spin" />
            ) : (
              Icon && <Icon size={15} />
            )}
            {showLabel && <span>{action.label}</span>}
          </button>
        );
      })}
    </div>
  );
}
