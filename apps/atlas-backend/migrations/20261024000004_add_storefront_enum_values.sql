/*
 * Storefront permissions. Own migration: a new enum value can't be used in
 * the transaction that adds it.
 */
ALTER TYPE atlas.permission ADD VALUE 'storefront.manage';
ALTER TYPE atlas.permission ADD VALUE 'promotion.manage';
ALTER TYPE atlas.permission ADD VALUE 'order.manage';
