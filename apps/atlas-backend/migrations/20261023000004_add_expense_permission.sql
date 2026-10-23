/*
 * FR-24 (Store Expense Management). Own migration, same reason as the
 * sales-permissions split: a new enum value can't be used in the
 * transaction that adds it.
 */
ALTER TYPE atlas.permission ADD VALUE 'expense.manage';
