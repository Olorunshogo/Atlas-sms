/*
 * FR-14. Own migration: a new enum value can't be used in the transaction
 * that adds it.
 */
ALTER TYPE atlas.stock_movement_type ADD VALUE 'write_off';
