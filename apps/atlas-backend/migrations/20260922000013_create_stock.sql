/*
 * "3. Stock item identity" (docs/Atlas/README.md). Some products track by
 * product alone (most retail goods), some by batch (pharmacy), some by
 * serial (phones), some by both. `atlas.stock_items` is the small
 * enumeration table that makes this fall out of data rather than special
 * cases in the sales code: a row here exists only when the product's type
 * has `identity_mode != 'by_product'`, and `batch_number`/`serial_number`
 * are populated according to that mode.
 *
 * Current on-hand quantity per store is NOT a column anywhere - it's the
 * running total of `stock_movements` for that (product, store[, stock_item])
 * triple, computed at read time or maintained by a later materialized
 * view/trigger. Storing a mutable "quantity" column invites it silently
 * drifting from the ledger that's supposed to be the source of truth.
 */
CREATE TABLE atlas.stock_items (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    batch_number TEXT,
    serial_number TEXT,
    expiry_date DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT stock_item_has_an_identity
        CHECK (batch_number IS NOT NULL OR serial_number IS NOT NULL)
);

CREATE UNIQUE INDEX stock_items_product_serial_idx
    ON atlas.stock_items (product_id, serial_number) WHERE serial_number IS NOT NULL;
CREATE INDEX stock_items_product_batch_idx
    ON atlas.stock_items (product_id, batch_number) WHERE batch_number IS NOT NULL;

ALTER TABLE atlas.stock_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.stock_items FORCE ROW LEVEL SECURITY;
CREATE POLICY stock_items_isolation ON atlas.stock_items
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === The ledger

CREATE TYPE atlas.stock_movement_type AS ENUM (
    'purchase_receipt', 'sale', 'sale_return', 'adjustment', 'transfer_out', 'transfer_in'
);

/*
 * Append-only - nothing about this table is ever UPDATEd or DELETEd by
 * application code (no updated_at/deleted_at columns, deliberately). Every
 * row's `reason` is mandatory (docs/Atlas/README.md: "adjustments with a
 * mandatory reason on every movement"). `quantity` is signed: positive for
 * stock coming in (purchase receipt, sale return, transfer in), negative
 * for stock going out (sale, transfer out) - on-hand is `sum(quantity)`
 * grouped by (product_id, store_id[, stock_item_id]).
 */
CREATE TABLE atlas.stock_movements (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    stock_item_id UUID REFERENCES atlas.stock_items(id),

    movement_type atlas.stock_movement_type NOT NULL,
    quantity NUMERIC NOT NULL CHECK (quantity != 0),
    reason TEXT NOT NULL,

    -- Loosely-typed pointer to whatever caused this movement (a sale, a
    -- purchase order, a transfer) - not a FK, since the referenced table
    -- varies by movement_type and this ledger predates all of them being
    -- built. Tightened once sales/purchasing land, if it's still needed.
    reference_id UUID,

    created_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX stock_movements_store_product_idx ON atlas.stock_movements (store_id, product_id);
CREATE INDEX stock_movements_stock_item_idx ON atlas.stock_movements (stock_item_id);
CREATE INDEX stock_movements_tenant_idx ON atlas.stock_movements (tenant_id);

ALTER TABLE atlas.stock_movements ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.stock_movements FORCE ROW LEVEL SECURITY;
CREATE POLICY stock_movements_isolation ON atlas.stock_movements
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
