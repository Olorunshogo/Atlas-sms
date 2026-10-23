/*
 * FR-08. Categories are free-form merchandising groups, separate from
 * product types (which carry the attribute schema and stock identity).
 * A variant is an ordinary product - own SKU, barcode, listing, price
 * rules and stock - that points at the product it varies.
 */
CREATE TABLE atlas.product_categories (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    archived_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, name)
);

ALTER TABLE atlas.product_categories ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.product_categories FORCE ROW LEVEL SECURITY;
CREATE POLICY product_categories_isolation ON atlas.product_categories
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

ALTER TABLE atlas.products
    ADD COLUMN category_id UUID REFERENCES atlas.product_categories(id),
    ADD COLUMN parent_product_id UUID REFERENCES atlas.products(id),
    ADD CONSTRAINT product_not_its_own_variant CHECK (parent_product_id <> id);

CREATE INDEX products_category_idx ON atlas.products (category_id) WHERE category_id IS NOT NULL;
CREATE INDEX products_parent_idx ON atlas.products (parent_product_id) WHERE parent_product_id IS NOT NULL;
