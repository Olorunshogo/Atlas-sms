-- FR-21: submit/cancel transition timestamps.
ALTER TABLE atlas.purchase_orders ADD COLUMN submitted_at TIMESTAMPTZ;
ALTER TABLE atlas.purchase_orders ADD COLUMN cancelled_at TIMESTAMPTZ;
ALTER TABLE atlas.purchase_orders ADD COLUMN cancelled_by_user_id UUID REFERENCES atlas.users(id);

-- FR-23: a return references the GRN line it reverses, not the order line.
CREATE TABLE atlas.purchase_returns (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    supplier_id UUID NOT NULL REFERENCES atlas.suppliers(id),
    purchase_order_id UUID NOT NULL REFERENCES atlas.purchase_orders(id),
    reason TEXT NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX purchase_returns_order_idx ON atlas.purchase_returns (purchase_order_id);

ALTER TABLE atlas.purchase_returns ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.purchase_returns FORCE ROW LEVEL SECURITY;
CREATE POLICY purchase_returns_isolation ON atlas.purchase_returns
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.purchase_return_lines (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    purchase_return_id UUID NOT NULL REFERENCES atlas.purchase_returns(id) ON DELETE CASCADE,
    goods_received_note_line_id UUID NOT NULL REFERENCES atlas.goods_received_note_lines(id),
    product_id UUID NOT NULL REFERENCES atlas.products(id),
    quantity NUMERIC NOT NULL CHECK (quantity > 0),
    unit_cost_amount_minor BIGINT NOT NULL,
    stock_movement_id UUID REFERENCES atlas.stock_movements(id)
);

CREATE INDEX purchase_return_lines_return_idx ON atlas.purchase_return_lines (purchase_return_id);

ALTER TABLE atlas.purchase_return_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.purchase_return_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY purchase_return_lines_isolation ON atlas.purchase_return_lines
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

ALTER TABLE atlas.goods_received_note_lines ADD COLUMN quantity_returned NUMERIC NOT NULL DEFAULT 0;
