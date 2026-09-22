/*
 * Atlas runs on its own Postgres server (not shared with atlas-backend),
 * and additionally keeps its own tables inside a dedicated `atlas` schema
 * rather than `public`, so a bug in one product's queries has no path to
 * the other's data even if the two ever end up on the same instance later.
 * Every migration after this one qualifies table names as `atlas.<table>`.
 */
CREATE SCHEMA IF NOT EXISTS atlas;
