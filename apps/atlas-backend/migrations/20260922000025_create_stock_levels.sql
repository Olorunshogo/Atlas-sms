/*
 * Current balance per (store, product[, stock_item]). This reverses the
 * earlier "on-hand is only ever SUM(stock_movements)" stance for one
 * reason: the ledger alone cannot enforce "never negative" at the database
 * level or serialize two concurrent sales on one row. A balance row can -
 * the CHECK below rejects any write that would go below zero, and the
 * UPDATE that decrements it takes the row lock that makes concurrent
 * sales queue up instead of overselling.
 *
 * The ledger stays the audit trail and the source of truth for history.
 * This table is only ever written in the same transaction as the
 * stock_movements row that explains the change (stock::service is the one
 * write path), so the two cannot drift; the integration tests assert
 * balance = SUM(ledger) after mixed operations.
 *
 * A product's overall total is never stored: it is SUM(quantity) over
 * these rows at read time.
 */
SET LOCAL app.bypass_tenant = 'on';

CREATE TABLE atlas.stock_levels (
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    stock_item_id UUID REFERENCES atlas.stock_items(id),

    /* NULL cannot sit in a primary key, so by-product rows (no batch or
     * serial) are keyed on the nil UUID instead. */
    item_key UUID GENERATED ALWAYS AS (
        coalesce(stock_item_id, '00000000-0000-0000-0000-000000000000'::uuid)
    ) STORED,

    quantity NUMERIC NOT NULL CHECK (quantity >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (store_id, product_id, item_key)
);

CREATE INDEX stock_levels_product_idx ON atlas.stock_levels (product_id);
CREATE INDEX stock_levels_tenant_idx ON atlas.stock_levels (tenant_id);

ALTER TABLE atlas.stock_levels ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.stock_levels FORCE ROW LEVEL SECURITY;
CREATE POLICY stock_levels_isolation ON atlas.stock_levels
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

/* Seed balances from whatever the ledger already holds. Fails loudly (the
 * CHECK) if existing history already sums negative anywhere. */
INSERT INTO atlas.stock_levels (tenant_id, store_id, product_id, stock_item_id, quantity)
SELECT tenant_id, store_id, product_id, stock_item_id, SUM(quantity)
FROM atlas.stock_movements
GROUP BY tenant_id, store_id, product_id, stock_item_id;
