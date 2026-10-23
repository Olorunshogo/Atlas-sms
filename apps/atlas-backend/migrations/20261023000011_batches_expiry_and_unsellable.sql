/*
 * FR-14. Batches are `stock_items` rows; one batch number per product so
 * receiving the same lot twice tops it up rather than splitting it.
 */
CREATE UNIQUE INDEX stock_items_product_batch_unique_idx
    ON atlas.stock_items (product_id, batch_number)
    WHERE batch_number IS NOT NULL AND serial_number IS NULL;

ALTER TABLE atlas.stores
    ADD COLUMN allow_expired_sale BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN expiry_warning_days INT NOT NULL DEFAULT 30 CHECK (expiry_warning_days >= 0);

/*
 * Goods taken out of saleable stock (write-offs) or never returned to it
 * (non-restockable returns), kept apart from the ledger's on-hand.
 */
CREATE TABLE atlas.unsellable_stock (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id),
    stock_item_id UUID REFERENCES atlas.stock_items(id),
    quantity NUMERIC NOT NULL CHECK (quantity > 0),
    condition atlas.item_condition NOT NULL,
    source TEXT NOT NULL,
    reference_id UUID,
    reason TEXT NOT NULL,
    recorded_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX unsellable_stock_store_idx ON atlas.unsellable_stock (store_id, created_at DESC);

ALTER TABLE atlas.unsellable_stock ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.unsellable_stock FORCE ROW LEVEL SECURITY;
CREATE POLICY unsellable_stock_isolation ON atlas.unsellable_stock
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
