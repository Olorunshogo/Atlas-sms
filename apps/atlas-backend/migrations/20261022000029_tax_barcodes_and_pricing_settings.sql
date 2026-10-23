/*
 * What the server needs to price a cart itself instead of trusting the
 * till: tax rates, barcodes, and the per-store pricing settings.
 */

-- === Tax rates

/*
 * Basis points, not a float or NUMERIC percentage: 750 is 7.5% VAT, and
 * every tax calculation stays in integer arithmetic until the single
 * rounding step in pricing::engine. An exempt product points at a 0 bp rate
 * rather than at NULL, so "exempt" is an explicit choice, not a missing one.
 */
CREATE TABLE atlas.tax_rates (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    rate_basis_points INT NOT NULL CHECK (rate_basis_points >= 0 AND rate_basis_points <= 10000),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, name)
);

ALTER TABLE atlas.tax_rates ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.tax_rates FORCE ROW LEVEL SECURITY;
CREATE POLICY tax_rates_isolation ON atlas.tax_rates
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- NULL tax_rate_id falls back to the store's default rate.
ALTER TABLE atlas.products
    ADD COLUMN tax_rate_id UUID REFERENCES atlas.tax_rates(id),
    ADD COLUMN is_returnable BOOLEAN NOT NULL DEFAULT true;

/*
 * prices_include_tax defaults to true: Nigerian retail shelf prices are
 * VAT-inclusive. cash_rounding_increment_minor = 1 means no cash rounding;
 * 500 rounds cash totals to the nearest N5. max_cashier_discount_basis_points
 * is how much discount (as a share of gross) anyone may give without
 * `sale.discount`.
 */
ALTER TABLE atlas.stores
    ADD COLUMN default_tax_rate_id UUID REFERENCES atlas.tax_rates(id),
    ADD COLUMN prices_include_tax BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN cash_rounding_increment_minor BIGINT NOT NULL DEFAULT 1 CHECK (cash_rounding_increment_minor >= 1),
    ADD COLUMN max_cashier_discount_basis_points INT NOT NULL DEFAULT 0
        CHECK (max_cashier_discount_basis_points >= 0 AND max_cashier_discount_basis_points <= 10000);

-- === Barcodes

/*
 * One product can carry several barcodes, each tied to the sale unit it
 * represents - scanning the carton barcode adds a carton, scanning the
 * single-unit barcode adds one unit. Unique per tenant so a scan always
 * resolves to exactly one product.
 */
CREATE TABLE atlas.product_barcodes (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    unit_id UUID NOT NULL REFERENCES atlas.units_of_measure(id),
    barcode TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, barcode)
);

CREATE INDEX product_barcodes_product_idx ON atlas.product_barcodes (product_id);

ALTER TABLE atlas.product_barcodes ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.product_barcodes FORCE ROW LEVEL SECURITY;
CREATE POLICY product_barcodes_isolation ON atlas.product_barcodes
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE INDEX price_rules_store_product_idx ON atlas.price_rules (store_id, product_id);
