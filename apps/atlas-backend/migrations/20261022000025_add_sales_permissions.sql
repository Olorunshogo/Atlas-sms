/*
 * Permissions for the sales, receipts, returns and customer work (FR-15 to
 * FR-19). Split finer than the original `sale.*` trio only where a real
 * role draws the line differently: a cashier may look customers up and
 * start a return, but not see purchase history or override a price.
 *
 * In its own migration, separate from the UPDATE that uses these values
 * (20261022000026): Postgres forbids using a newly added enum value in the
 * same transaction that added it.
 */
ALTER TYPE atlas.permission ADD VALUE 'sale.view';
ALTER TYPE atlas.permission ADD VALUE 'sale.discount';
ALTER TYPE atlas.permission ADD VALUE 'sale.price_override';
ALTER TYPE atlas.permission ADD VALUE 'sale.return';
ALTER TYPE atlas.permission ADD VALUE 'receipt.reprint';
ALTER TYPE atlas.permission ADD VALUE 'customer.view';
ALTER TYPE atlas.permission ADD VALUE 'customer.manage';
ALTER TYPE atlas.permission ADD VALUE 'customer.history';
ALTER TYPE atlas.permission ADD VALUE 'payment.manage';
