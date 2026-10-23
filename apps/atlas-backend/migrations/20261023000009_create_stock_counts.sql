/*
 * FR-12 physical counts. A count records what was counted next to what the
 * ledger said at that moment; nothing moves until it is approved, and then
 * only the variance is posted as adjustments referencing the count.
 */
CREATE TYPE atlas.stock_count_status AS ENUM ('pending', 'approved', 'rejected');

CREATE TABLE atlas.stock_counts (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    status atlas.stock_count_status NOT NULL DEFAULT 'pending',
    reason TEXT NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_by_user_id UUID REFERENCES atlas.users(id),
    decided_at TIMESTAMPTZ
);

CREATE INDEX stock_counts_store_idx ON atlas.stock_counts (store_id, created_at DESC);

ALTER TABLE atlas.stock_counts ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.stock_counts FORCE ROW LEVEL SECURITY;
CREATE POLICY stock_counts_isolation ON atlas.stock_counts
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.stock_count_lines (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    stock_count_id UUID NOT NULL REFERENCES atlas.stock_counts(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id),
    stock_item_id UUID REFERENCES atlas.stock_items(id),
    counted_quantity NUMERIC NOT NULL CHECK (counted_quantity >= 0),
    recorded_quantity NUMERIC NOT NULL,
    variance NUMERIC GENERATED ALWAYS AS (counted_quantity - recorded_quantity) STORED
);

CREATE UNIQUE INDEX stock_count_lines_item_idx
    ON atlas.stock_count_lines (stock_count_id, product_id, coalesce(stock_item_id, '00000000-0000-0000-0000-000000000000'));

ALTER TABLE atlas.stock_count_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.stock_count_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY stock_count_lines_isolation ON atlas.stock_count_lines
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
