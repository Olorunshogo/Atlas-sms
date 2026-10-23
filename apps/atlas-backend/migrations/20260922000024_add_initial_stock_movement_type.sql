/*
 * In its own migration, separate from anything that uses it: Postgres
 * forbids using a newly added enum value in the same transaction that
 * added it, and sqlx runs each migration file in its own transaction.
 * `purchase_receipt` already covers restocks, so the only gap in the
 * ledger vocabulary is the first stock a product ever gets at a store.
 */
ALTER TYPE atlas.stock_movement_type ADD VALUE 'initial_stock';
