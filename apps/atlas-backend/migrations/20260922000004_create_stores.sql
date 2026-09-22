/*
 * One row per branch. `store_group_id` is nullable and unused until the V2
 * cross-store-grant work (docs/v2.md) - kept here now only so adding it
 * later is a column-populate, not a migration that touches every existing
 * row's shape.
 */
CREATE TABLE atlas.stores (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_group_id UUID,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    currency CHAR(3) NOT NULL DEFAULT 'NGN',
    timezone TEXT NOT NULL DEFAULT 'Africa/Lagos',
    address TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,

    UNIQUE (tenant_id, slug)
);

CREATE INDEX stores_tenant_idx ON atlas.stores (tenant_id);
