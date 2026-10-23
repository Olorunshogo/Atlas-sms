/*
 * FR-09 price history. One row per selling or cost price change, with
 * the price it replaced. `price_rule_id` has no foreign key: a deleted
 * rule's history outlives it.
 */
CREATE TYPE atlas.price_kind AS ENUM ('selling', 'cost');

CREATE TABLE atlas.price_changes (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    store_id UUID REFERENCES atlas.stores(id) ON DELETE CASCADE,
    customer_group_id UUID REFERENCES atlas.customer_groups(id) ON DELETE CASCADE,
    min_quantity NUMERIC,
    kind atlas.price_kind NOT NULL,
    previous_amount_minor BIGINT,
    new_amount_minor BIGINT,
    price_rule_id UUID,
    effective_from TIMESTAMPTZ NOT NULL,
    changed_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX price_changes_product_idx ON atlas.price_changes (product_id, changed_at DESC);

ALTER TABLE atlas.price_changes ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.price_changes FORCE ROW LEVEL SECURITY;
CREATE POLICY price_changes_isolation ON atlas.price_changes
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
