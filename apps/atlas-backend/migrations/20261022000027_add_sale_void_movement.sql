/*
 * A voided sale puts its stock back under its own movement type rather
 * than reusing `sale_return`, so reports can tell "the till cancelled this"
 * from "a customer brought it back". Own migration: a new enum value can't
 * be used in the transaction that adds it.
 */
ALTER TYPE atlas.stock_movement_type ADD VALUE 'sale_void';
