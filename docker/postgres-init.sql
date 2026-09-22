/*
 * Runs once on first container start, before any sqlx migration. Creates
 * only the login role; the privilege grant lives in the RLS migration,
 * which is what actually knows about klassivox_app.
 *
 * This role is deliberately NOT a superuser and NOT the database owner.
 * Both bypass row level security, which would silently disable every
 * tenant isolation policy in the schema.
 *
 * Local development credential only. Staging and production create this
 * role through the provider with a real secret.
 */
CREATE ROLE klassivox_api LOGIN PASSWORD 'klassivox_api_dev';
GRANT CONNECT ON DATABASE atlas TO klassivox_api;
