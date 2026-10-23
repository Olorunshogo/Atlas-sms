-- FR-22 staged transfer workflow: request, approve/reject, dispatch, receive, cancel.
CREATE TYPE atlas.stock_transfer_status AS ENUM (
    'requested', 'approved', 'rejected', 'dispatched', 'received', 'cancelled'
);

CREATE TABLE atlas.stock_transfers (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    from_store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    to_store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    stock_item_id UUID REFERENCES atlas.stock_items(id),
    quantity NUMERIC NOT NULL CHECK (quantity > 0),
    reason TEXT NOT NULL,
    status atlas.stock_transfer_status NOT NULL DEFAULT 'requested',

    requested_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    approved_by_user_id UUID REFERENCES atlas.users(id),
    approved_at TIMESTAMPTZ,
    dispatched_at TIMESTAMPTZ,
    received_at TIMESTAMPTZ,
    cancelled_by_user_id UUID REFERENCES atlas.users(id),
    cancelled_at TIMESTAMPTZ,

    CONSTRAINT stock_transfer_stores_differ CHECK (from_store_id != to_store_id)
);

CREATE INDEX stock_transfers_tenant_status_idx
    ON atlas.stock_transfers (tenant_id, status);

CREATE INDEX stock_transfers_from_store_idx ON atlas.stock_transfers (from_store_id);
CREATE INDEX stock_transfers_to_store_idx ON atlas.stock_transfers (to_store_id);

ALTER TABLE atlas.stock_transfers ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.stock_transfers FORCE ROW LEVEL SECURITY;
CREATE POLICY stock_transfers_isolation ON atlas.stock_transfers
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
