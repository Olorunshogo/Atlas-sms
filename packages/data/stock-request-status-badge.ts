import type { StockRequestStatus } from '@atlas/reference-domain-status';

/*
 * The one definition of how a stock request's status reads as a badge, so
 * request-stock.tsx and overview.tsx's "Recent product requests" card cannot
 * disagree about what "Processing" looks like.
 */
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
