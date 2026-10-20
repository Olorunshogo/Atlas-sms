/*
 * `attributes` holds whatever `product_types.attribute_schema` for this
 * product's type declares - no per-vertical columns (docs/Atlas/README.md).
 * No price column here: price is the resolved answer from `price_rules`
 * (migration 20260922000016), never a bare field on the product. What IS
 * a real product-level fact is its base unit and its sale-unit
 * conversions (this migration) and its per-store listing/cost (next one).
 */
CREATE TABLE atlas.products (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    product_type_id UUID NOT NULL REFERENCES atlas.product_types(id),
    base_unit_id UUID NOT NULL REFERENCES atlas.units_of_measure(id),

    name TEXT NOT NULL,
    sku TEXT,
    attributes JSONB NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,

    UNIQUE (tenant_id, sku)
);

CREATE INDEX products_tenant_idx ON atlas.products (tenant_id);
CREATE INDEX products_type_idx ON atlas.products (product_type_id);
CREATE INDEX products_attributes_idx ON atlas.products USING gin (attributes);

ALTER TABLE atlas.products ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.products FORCE ROW LEVEL SECURITY;
CREATE POLICY products_isolation ON atlas.products
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- Alternate sale units beyond the base (e.g. base = kg, sale unit = 50kg
-- bag at factor_to_base = 50). Quantity is NUMERIC throughout, never
-- integer - a half-bag sale is 0.5, not a rounding error. `tenant_id` is
-- denormalized from `products` (same pattern as every other RLS-scoped
-- table here) so the isolation policy doesn't need a join.
CREATE TABLE atlas.product_sale_units (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    unit_id UUID NOT NULL REFERENCES atlas.units_of_measure(id),
    factor_to_base NUMERIC NOT NULL CHECK (factor_to_base > 0),

    UNIQUE (product_id, unit_id)
);

ALTER TABLE atlas.product_sale_units ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.product_sale_units FORCE ROW LEVEL SECURITY;
CREATE POLICY product_sale_units_isolation ON atlas.product_sale_units
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- Per-store presence: not every branch stocks every product. Cost price is
-- genuinely per-store (what that branch paid its supplier); selling price
-- is not stored here at all - see price_rules. Money is amount_minor, not
-- a bare NUMERIC - never a currency-baked column name; the store's own `currency` column
-- is what it's minor units of.
CREATE TABLE atlas.product_store_listings (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    cost_amount_minor BIGINT,
    low_stock_threshold NUMERIC NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,

    UNIQUE (product_id, store_id)
);

CREATE INDEX product_store_listings_store_idx ON atlas.product_store_listings (store_id);

ALTER TABLE atlas.product_store_listings ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.product_store_listings FORCE ROW LEVEL SECURITY;
CREATE POLICY product_store_listings_isolation ON atlas.product_store_listings
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
