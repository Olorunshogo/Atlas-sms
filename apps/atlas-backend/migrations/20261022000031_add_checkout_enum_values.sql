/*
 * Enum values the checkout, payment lifecycle and refund work needs. Own
 * migration because Postgres won't let a new enum value be used in the
 * transaction that adds it; 20261022000032 onward use them.
 *
 * - sale_status 'awaiting_payment': a sale whose lines are recorded but
 *   whose payment (e.g. a card terminal) hasn't confirmed yet. Stock is not
 *   deducted until it completes.
 * - payment_status 'reversed' (cancelled same-day, e.g. a voided sale),
 *   'partially_refunded' and 'refunded' (money returned after the fact).
 * - payment_method 'store_credit' and 'other' (tenant-defined methods such
 *   as a specific POS terminal or mobile wallet).
 */
ALTER TYPE atlas.sale_status ADD VALUE 'awaiting_payment';
ALTER TYPE atlas.payment_status ADD VALUE 'reversed';
ALTER TYPE atlas.payment_status ADD VALUE 'partially_refunded';
ALTER TYPE atlas.payment_status ADD VALUE 'refunded';
ALTER TYPE atlas.payment_method ADD VALUE 'store_credit';
ALTER TYPE atlas.payment_method ADD VALUE 'other';
