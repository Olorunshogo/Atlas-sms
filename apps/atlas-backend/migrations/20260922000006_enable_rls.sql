/*
 * Row Level Security, same fail-closed mechanism as Klassivox's
 * TenantTx/RLS pair, renamed for this product: `atlas_app` (privilege
 * holder) / `atlas_api` (the login role the app actually connects as).
 * These names deliberately do not collide with klassivox_app/klassivox_api
 * even though Atlas runs on its own server, in case the two ever share an
 * instance later.
 *
 * Policies read two transaction-local settings the Rust TenantTx
 * equivalent sets via set_config(..., true) immediately after BEGIN:
 *
 *   app.current_tenant_id
 *   app.current_store_id
 *
 * current_setting(name, true) returns NULL when unset, so an unscoped
 * connection matches no rows - a handler that skips the extractor sees an
 * empty database rather than another tenant's rows.
 *
 * app.bypass_tenant = 'on' is the platform-role escape hatch (AdminTx),
 * set only after a platform_admin check has already passed in app code.
 */
CREATE ROLE atlas_app NOLOGIN;

GRANT USAGE ON SCHEMA atlas TO atlas_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA atlas TO atlas_app;
ALTER DEFAULT PRIVILEGES IN SCHEMA atlas
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO atlas_app;

-- FORCE is required in addition to ENABLE, or the table owner (who runs
-- migrations) is exempt from its own policies.

ALTER TABLE atlas.tenants ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.tenants FORCE ROW LEVEL SECURITY;
CREATE POLICY tenants_isolation ON atlas.tenants
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

ALTER TABLE atlas.stores ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.stores FORCE ROW LEVEL SECURITY;
CREATE POLICY stores_isolation ON atlas.stores
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

ALTER TABLE atlas.role_grants ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.role_grants FORCE ROW LEVEL SECURITY;
CREATE POLICY role_grants_isolation ON atlas.role_grants
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
        OR scope = 'platform'
    );

/*
 * atlas.roles carries no isolation predicate on `tenant_id` beyond what the
 * app layer already needs to enforce (system rows are tenant_id IS NULL and
 * meant to be readable by everyone; a tenant's own custom roles should only
 * be visible to that tenant). Modeled as a policy rather than left open:
 */
ALTER TABLE atlas.roles ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.roles FORCE ROW LEVEL SECURITY;
CREATE POLICY roles_isolation ON atlas.roles
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id IS NULL
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

/*
 * atlas.users carries no policy on purpose, same reasoning as Klassivox:
 * not tenant-scoped (one human, many role grants across many tenants),
 * reachable only through atlas.role_grants, which is policed above.
 */

/*
 * atlas_app is NOLOGIN and holds privileges only. The role the app connects
 * as must be a separate non-superuser login role granted membership here -
 * a superuser bypasses RLS entirely and would make every policy above
 * decorative. No credential is created by this migration; the login role is
 * created per environment (docker-compose's init script locally, the
 * provider's console in staging/production) and this block grants it
 * membership if it happens to already exist.
 */
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'atlas_api') THEN
        GRANT atlas_app TO atlas_api;
    END IF;
END
$$;
