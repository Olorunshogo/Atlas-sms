/*
 * The core mechanism absorbing store-type variation (docs/Atlas/README.md
 * "1. Product types with attribute schemas"). A product belongs to a
 * product type; the type declares its attributes (name, data type,
 * required, searchable, identity-bearing) as data, not columns. No
 * expiry/size/serial/grade columns anywhere - a pharmacy's "expiry_date"
 * and a boutique's "size" are both just rows in `attribute_schema`.
 *
 * `attribute_schema` shape (validated in the service layer, not by a DB
 * constraint - Postgres has no JSON-schema-of-a-jsonb-column check):
 *   [{ "key": "expiry_date", "label": "Expiry date", "type": "date",
 *      "required": true, "searchable": true, "identity": true }, ...]
 */
CREATE TYPE atlas.stock_identity_mode AS ENUM ('by_product', 'by_batch', 'by_serial', 'by_both');

CREATE TABLE atlas.product_types (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    attribute_schema JSONB NOT NULL DEFAULT '[]',

    -- Which stock movements must carry identifying values. A pharmacy's
    -- controlled drugs are by_batch, a phone shop's units are by_serial;
    -- most retail goods are by_product (no per-unit identity at all).
    identity_mode atlas.stock_identity_mode NOT NULL DEFAULT 'by_product',

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,

    UNIQUE (tenant_id, name)
);

CREATE INDEX product_types_tenant_idx ON atlas.product_types (tenant_id);

ALTER TABLE atlas.product_types ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.product_types FORCE ROW LEVEL SECURITY;
CREATE POLICY product_types_isolation ON atlas.product_types
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
