/*
 * FR-20 supplier records, products and payments; FR-21 approval, charges,
 * cancelled quantities and over-receipt exceptions; FR-23 returns that
 * wait for confirmation.
 */

-- === FR-20

ALTER TABLE atlas.suppliers
    ADD COLUMN code TEXT,
    ADD COLUMN address TEXT,
    ADD COLUMN payment_terms_days INT CHECK (payment_terms_days >= 0),
    ADD COLUMN deactivated_at TIMESTAMPTZ;

CREATE UNIQUE INDEX suppliers_tenant_code_idx ON atlas.suppliers (tenant_id, lower(code)) WHERE code IS NOT NULL;

CREATE TABLE atlas.supplier_products (
    supplier_id UUID NOT NULL REFERENCES atlas.suppliers(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    supplier_sku TEXT,
    unit_cost_amount_minor BIGINT CHECK (unit_cost_amount_minor >= 0),

    PRIMARY KEY (supplier_id, product_id)
);

ALTER TABLE atlas.supplier_products ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.supplier_products FORCE ROW LEVEL SECURITY;
CREATE POLICY supplier_products_isolation ON atlas.supplier_products
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- Money paid to a supplier; each one lowers the outstanding balance
CREATE TABLE atlas.supplier_payments (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    supplier_id UUID NOT NULL REFERENCES atlas.suppliers(id),
    amount_minor BIGINT NOT NULL CHECK (amount_minor > 0),
    method TEXT NOT NULL,
    reference TEXT,
    paid_on DATE NOT NULL,
    recorded_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX supplier_payments_supplier_idx ON atlas.supplier_payments (supplier_id, created_at DESC);

ALTER TABLE atlas.supplier_payments ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.supplier_payments FORCE ROW LEVEL SECURITY;
CREATE POLICY supplier_payments_isolation ON atlas.supplier_payments
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === FR-21

ALTER TABLE atlas.purchase_orders
    ADD COLUMN approved_by_user_id UUID REFERENCES atlas.users(id),
    ADD COLUMN approved_at TIMESTAMPTZ,
    -- Charges are owed with the first receipt, once
    ADD COLUMN charges_posted_at TIMESTAMPTZ;

ALTER TABLE atlas.purchase_order_lines
    ADD COLUMN quantity_cancelled NUMERIC NOT NULL DEFAULT 0 CHECK (quantity_cancelled >= 0);

CREATE TABLE atlas.purchase_order_charges (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    purchase_order_id UUID NOT NULL REFERENCES atlas.purchase_orders(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    amount_minor BIGINT NOT NULL CHECK (amount_minor > 0)
);

CREATE INDEX purchase_order_charges_order_idx ON atlas.purchase_order_charges (purchase_order_id);

ALTER TABLE atlas.purchase_order_charges ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.purchase_order_charges FORCE ROW LEVEL SECURITY;
CREATE POLICY purchase_order_charges_isolation ON atlas.purchase_order_charges
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- Set only when an approver received more than was outstanding
ALTER TABLE atlas.goods_received_note_lines ADD COLUMN over_receipt_reason TEXT;

-- The order's charges, carried by whichever receipt came first
ALTER TABLE atlas.goods_received_notes ADD COLUMN charges_amount_minor BIGINT NOT NULL DEFAULT 0;

UPDATE atlas.roles
SET permissions = permissions || ARRAY['purchase.approve']::atlas.permission[]
WHERE is_system AND name IN ('owner', 'admin', 'store_manager', 'platform_admin');

-- === FR-23

CREATE TYPE atlas.purchase_return_status AS ENUM ('pending', 'confirmed', 'cancelled');

-- Existing returns were posted on creation, so they are already confirmed
ALTER TABLE atlas.purchase_returns
    ADD COLUMN status atlas.purchase_return_status NOT NULL DEFAULT 'confirmed',
    ADD COLUMN confirmed_by_user_id UUID REFERENCES atlas.users(id),
    ADD COLUMN confirmed_at TIMESTAMPTZ,
    ADD COLUMN cancelled_at TIMESTAMPTZ;

ALTER TABLE atlas.purchase_returns ALTER COLUMN status SET DEFAULT 'pending';
