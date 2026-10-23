/*
 * Supporting schema for the central inventory transaction service
 * (src/inventory). Every stock change now goes through one writer that
 * converts sale units to the base unit, serialises concurrent writers per
 * (store, product) with an advisory lock, and applies a negative-stock
 * policy. On-hand stays derived from the ledger; nothing here adds a
 * mutable quantity column.
 */

-- What was actually entered (2 bags) next to the base quantity the ledger
-- sums (100 kg). NULL on rows written before this migration and on
-- movements entered directly in the base unit.
ALTER TABLE atlas.stock_movements
    ADD COLUMN unit_id UUID REFERENCES atlas.units_of_measure(id),
    ADD COLUMN unit_quantity NUMERIC;

CREATE INDEX stock_movements_store_product_item_idx
    ON atlas.stock_movements (store_id, product_id, stock_item_id);

-- Off by default: a live sale that would take a store below zero is
-- rejected. A store that genuinely sells ahead of receiving (made-to-order,
-- late GRN entry) can opt in.
ALTER TABLE atlas.stores
    ADD COLUMN allow_negative_stock BOOLEAN NOT NULL DEFAULT false;

/*
 * Review queue for stock the ledger says wasn't there. An offline sale is
 * accepted on sync even when it oversells - the goods already left the
 * shop - and each shortfall lands here for a manager to investigate
 * (miscount, theft, unrecorded receipt) instead of disappearing into a
 * negative balance nobody looks at.
 */
CREATE TABLE atlas.stock_exceptions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    stock_item_id UUID REFERENCES atlas.stock_items(id),
    movement_id UUID NOT NULL REFERENCES atlas.stock_movements(id),
    on_hand_before NUMERIC NOT NULL,
    requested NUMERIC NOT NULL,
    source TEXT NOT NULL,
    reference_id UUID,
    resolved_at TIMESTAMPTZ,
    resolved_by_user_id UUID REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX stock_exceptions_store_open_idx ON atlas.stock_exceptions (store_id) WHERE resolved_at IS NULL;

ALTER TABLE atlas.stock_exceptions ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.stock_exceptions FORCE ROW LEVEL SECURITY;
CREATE POLICY stock_exceptions_isolation ON atlas.stock_exceptions
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
