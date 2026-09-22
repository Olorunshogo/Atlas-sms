/*
 * Runs once on first container start, before any sqlx migration, on
 * Atlas's own Postgres server - not the container that backs
 * atlas-backend. Creates only the login role; the privilege grant
 * lives in the RLS migration, which is what actually knows about
 * atlas_app.
 *
 * Not a superuser and not the database owner - both bypass row level
 * security, which would silently disable every tenant isolation policy.
 *
 * Local development credential only. Staging and production create this
 * role through the provider with a real secret.
 */
CREATE ROLE atlas_api LOGIN PASSWORD 'atlas_api_dev';
GRANT CONNECT ON DATABASE atlas TO atlas_api;
