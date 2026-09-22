/*
 * "Pricing, which sits slightly apart" (docs/Atlas/README.md). Price is
 * never a column on a product - it's the resolved answer to "this
 * product, this store, this customer group, this quantity, this date".
 * Branch pricing, wholesale tiers and promotions are all just rows here,
 * not separate mechanisms.
 *
 * NULL on a scoping column means "applies to all" (store_id NULL = every
 * store, customer_group_id NULL = every customer). Resolution (most
 * specific match wins, ties broken by `priority` then by longest-running
 * rule) lives in the service layer, not as a SQL constraint - a
 * `CHECK`/index can't express "pick the best of several matching rows".
 */
CREATE TABLE atlas.customer_groups (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,

    UNIQUE (tenant_id, name)
);

ALTER TABLE atlas.customer_groups ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.customer_groups FORCE ROW LEVEL SECURITY;
CREATE POLICY customer_groups_isolation ON atlas.customer_groups
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.price_rules (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    store_id UUID REFERENCES atlas.stores(id) ON DELETE CASCADE,
    customer_group_id UUID REFERENCES atlas.customer_groups(id) ON DELETE CASCADE,

    min_quantity NUMERIC NOT NULL DEFAULT 1,

    -- amount_minor, not a bare NUMERIC - see product_store_listings'
    -- comment on cost_amount_minor for why. The applicable store's
    -- `currency` column is what this is minor units of; a rule with
    -- store_id NULL (applies to every store) assumes one currency across
    -- the tenant, which holds at Atlas's stated 1-20-stores-per-tenant
    -- scale and is revisited if a genuinely multi-currency tenant shows up.
    unit_price_amount_minor BIGINT NOT NULL,

    -- Explicit resolution weight for two rules that would otherwise tie
    -- (same product/store/group/quantity band) - higher wins.
    priority INT NOT NULL DEFAULT 0,

    starts_at TIMESTAMPTZ,
    ends_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX price_rules_lookup_idx ON atlas.price_rules (product_id, store_id, customer_group_id);

ALTER TABLE atlas.price_rules ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.price_rules FORCE ROW LEVEL SECURITY;
CREATE POLICY price_rules_isolation ON atlas.price_rules
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
