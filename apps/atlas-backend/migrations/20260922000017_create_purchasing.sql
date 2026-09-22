CREATE TABLE atlas.suppliers (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    phone TEXT,
    email TEXT,
    -- Positive = tenant owes the supplier. Adjusted by purchase orders
    -- (increases) and recorded payments (decreases) in the service layer,
    -- not a value the API sets directly.
    balance_amount_minor BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX suppliers_tenant_idx ON atlas.suppliers (tenant_id);

ALTER TABLE atlas.suppliers ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.suppliers FORCE ROW LEVEL SECURITY;
CREATE POLICY suppliers_isolation ON atlas.suppliers
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TYPE atlas.purchase_order_status AS ENUM ('draft', 'ordered', 'partially_received', 'received', 'cancelled');

CREATE TABLE atlas.purchase_orders (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    supplier_id UUID NOT NULL REFERENCES atlas.suppliers(id),
    status atlas.purchase_order_status NOT NULL DEFAULT 'draft',
    created_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX purchase_orders_store_idx ON atlas.purchase_orders (store_id);
CREATE INDEX purchase_orders_supplier_idx ON atlas.purchase_orders (supplier_id);

ALTER TABLE atlas.purchase_orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.purchase_orders FORCE ROW LEVEL SECURITY;
CREATE POLICY purchase_orders_isolation ON atlas.purchase_orders
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.purchase_order_lines (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    purchase_order_id UUID NOT NULL REFERENCES atlas.purchase_orders(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id),
    quantity_ordered NUMERIC NOT NULL CHECK (quantity_ordered > 0),
    quantity_received NUMERIC NOT NULL DEFAULT 0,
    unit_cost_amount_minor BIGINT NOT NULL
);

CREATE INDEX purchase_order_lines_po_idx ON atlas.purchase_order_lines (purchase_order_id);

ALTER TABLE atlas.purchase_order_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.purchase_order_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY purchase_order_lines_isolation ON atlas.purchase_order_lines
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

/*
 * A GRN records what was physically received, separate from what was
 * ordered (`purchase_order_lines.quantity_ordered`) - short shipments and
 * substitutions are the normal case, not an edge case, in this business.
 * Accepting a GRN is what actually writes a `stock_movements` row
 * (`movement_type = 'purchase_receipt'`), never the purchase order itself.
 */
CREATE TABLE atlas.goods_received_notes (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    purchase_order_id UUID NOT NULL REFERENCES atlas.purchase_orders(id),
    received_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE atlas.goods_received_notes ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.goods_received_notes FORCE ROW LEVEL SECURITY;
CREATE POLICY goods_received_notes_isolation ON atlas.goods_received_notes
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.goods_received_note_lines (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    goods_received_note_id UUID NOT NULL REFERENCES atlas.goods_received_notes(id) ON DELETE CASCADE,
    purchase_order_line_id UUID NOT NULL REFERENCES atlas.purchase_order_lines(id),
    quantity_received NUMERIC NOT NULL CHECK (quantity_received > 0),
    stock_movement_id UUID REFERENCES atlas.stock_movements(id)
);

CREATE INDEX grn_lines_grn_idx ON atlas.goods_received_note_lines (goods_received_note_id);

ALTER TABLE atlas.goods_received_note_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.goods_received_note_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY grn_lines_isolation ON atlas.goods_received_note_lines
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
