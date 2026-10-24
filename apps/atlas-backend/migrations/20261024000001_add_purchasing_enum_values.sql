/*
 * FR-21, FR-26, FR-27 and FR-29. Own migration: a new enum value can't be used in the
 * transaction that adds it.
 */
ALTER TYPE atlas.purchase_order_status ADD VALUE 'submitted' BEFORE 'ordered';
ALTER TYPE atlas.permission ADD VALUE 'purchase.approve';
ALTER TYPE atlas.notification_kind ADD VALUE 'purchase_order_approved';

-- FR-27 and FR-29
ALTER TYPE atlas.permission ADD VALUE 'audit.view';
ALTER TYPE atlas.permission ADD VALUE 'stock.approve';
