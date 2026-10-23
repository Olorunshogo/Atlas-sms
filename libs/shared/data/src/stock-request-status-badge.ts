/*
 * From the reference source - a field-agent stock request workflow (pending -> approved
 * -> processing -> ready_for_collection -> fulfilled), not Atlas's own
 * inventory/transfer model (see docs/atlas/CONTEXT.md). The original import
 * (`the reference domain-status package`) is a workspace package that
 * doesn't exist here - the status union is inlined below as a local
 * placeholder so this type-checks. Prune candidate.
 */

type StockRequestStatus =
  | 'pending'
  | 'approved'
  | 'processing'
  | 'ready_for_collection'
  | 'fulfilled';

export const STOCK_REQUEST_STATUS_BADGE: Record<
  StockRequestStatus,
  { bgClass: string; textClass: string; label: string }
> = {
  pending: {
    bgClass: 'bg-status-pending',
    textClass: 'text-status-pending-fg',
    label: 'Pending',
  },
  approved: {
    bgClass: 'bg-accent-soft',
    textClass: 'text-accent-soft-fg',
    label: 'Approved',
  },
  processing: {
    bgClass: 'bg-accent-soft',
    textClass: 'text-accent-soft-fg',
    label: 'Processing',
  },
  ready_for_collection: {
    bgClass: 'bg-status-active',
    textClass: 'text-status-active-fg',
    label: 'Ready for Collection',
  },
  fulfilled: {
    bgClass: 'bg-status-delivered',
    textClass: 'text-status-delivered-fg',
    label: 'Collected',
  },
};
