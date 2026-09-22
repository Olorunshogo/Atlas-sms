/*
 * One permission covers creating/editing products, product types, units
 * and price rules, rather than one per CRUD action - keeps the catalogue
 * small; split further only if a real case needs "can edit prices but not
 * product types" as a distinct grant.
 *
 * In its own migration, separate from the UPDATE that uses it
 * (20260922000021): Postgres forbids using a newly added enum value in the
 * same transaction that added it, and sqlx runs each migration file in its
 * own transaction.
 */
ALTER TYPE atlas.permission ADD VALUE 'product.manage';
