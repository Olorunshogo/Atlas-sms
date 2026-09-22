/*
 * Dynamic roles, not a fixed ladder. `atlas.roles` holds both the seeded
 * system roles (tenant_id NULL, is_system true, never editable) and any
 * custom role a tenant builds ('Regional Inventory Lead' etc). Permissions
 * are a Postgres enum array rather than free text, so a typo'd permission
 * string is rejected at write time instead of silently checking false at
 * request time.
 *
 * A grant is (user, role, tenant, store): store_id NULL on a tenant-scope
 * grant means tenant-wide (owner/admin see every store). Multiple roles per
 * user across different stores fall out of this being one row per grant,
 * not a single role column on the user.
 */
CREATE TYPE atlas.permission AS ENUM (
    'sale.create', 'sale.void', 'sale.refund',
    'stock.view', 'stock.adjust', 'stock.transfer',
    'purchase.create', 'supplier.manage',
    'staff.invite', 'staff.remove',
    'store.create', 'store.update',
    'role.create', 'role.assign',
    'report.view',
    'tenant.manage'
);

CREATE TYPE atlas.role_scope AS ENUM ('platform', 'tenant', 'store');
CREATE TYPE atlas.role_grant_status AS ENUM ('pending', 'active', 'revoked');

CREATE TABLE atlas.roles (
    id UUID PRIMARY KEY,
    tenant_id UUID REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    permissions atlas.permission[] NOT NULL DEFAULT '{}',
    is_system BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Two partial indexes because a plain UNIQUE(tenant_id, name) lets every
-- tenant-less (system) row share NULL, which Postgres treats as distinct.
CREATE UNIQUE INDEX roles_system_name_idx ON atlas.roles (name) WHERE tenant_id IS NULL;
CREATE UNIQUE INDEX roles_tenant_name_idx ON atlas.roles (tenant_id, name) WHERE tenant_id IS NOT NULL;

CREATE TABLE atlas.role_grants (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES atlas.users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES atlas.roles(id),
    scope atlas.role_scope NOT NULL,
    tenant_id UUID REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID REFERENCES atlas.stores(id) ON DELETE CASCADE,
    status atlas.role_grant_status NOT NULL DEFAULT 'pending',
    granted_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,

    UNIQUE (user_id, role_id, tenant_id, store_id),
    CONSTRAINT platform_grant_has_no_tenant
        CHECK (scope != 'platform' OR tenant_id IS NULL),
    CONSTRAINT tenant_scoped_grant_has_no_store
        CHECK (scope != 'tenant' OR store_id IS NULL),
    CONSTRAINT store_grant_requires_store
        CHECK (scope != 'store' OR store_id IS NOT NULL)
);

CREATE INDEX role_grants_user_idx ON atlas.role_grants (user_id);
CREATE INDEX role_grants_tenant_idx ON atlas.role_grants (tenant_id);
CREATE INDEX role_grants_store_idx ON atlas.role_grants (store_id);

-- V2: cross-store access for one grant without going full tenant-wide.
-- The "regional manager over 3 of my 5 stores" case. Table exists now so
-- the FK shape is settled; nothing writes to it until docs/v2.md's
-- multi-store phase.
CREATE TABLE atlas.store_group_grants (
    role_grant_id UUID NOT NULL REFERENCES atlas.role_grants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    PRIMARY KEY (role_grant_id, store_id)
);
