/*
 * Point of sale, offline-first from the first commit (docs/Atlas/README.md
 * "Offline", and docs/atlas-sms-schema-notes.md). Three things load-bear
 * here:
 *
 *   1. `id` is the CLIENT-generated sale id (UUIDv7, minted on the POS
 *      device before it ever reaches the network), not server-generated -
 *      it doubles as the idempotency key for `POST /sync/sales`: syncing
 *      the same sale twice (a flaky connection retried) is a harmless
 *      upsert on this primary key, not a duplicate sale.
 *   2. `device_id` is mandatory on every sale, so a sync conflict or an
 *      audit question ("which till rang this up") never depends on a
 *      cashier's memory.
 *   3. Stock is adjusted only when the server accepts the sale (a
 *      `stock_movements` row with `movement_type = 'sale'` and
 *      `reference_id = sales.id`) - never optimistically on the device.
 */
CREATE TYPE atlas.shift_status AS ENUM ('open', 'closed');

CREATE TABLE atlas.shifts (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,

    opened_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    opened_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    opening_cash_amount_minor BIGINT NOT NULL DEFAULT 0,

    closed_by_user_id UUID REFERENCES atlas.users(id),
    closed_at TIMESTAMPTZ,
    -- Counted by the cashier at close; reconciliation is
    -- closing_cash_amount_minor minus (opening + cash sales in the shift),
    -- computed in the service layer, not stored.
    closing_cash_amount_minor BIGINT,

    status atlas.shift_status NOT NULL DEFAULT 'open'
);

CREATE INDEX shifts_store_idx ON atlas.shifts (store_id);
-- At most one open shift per store at a time.
CREATE UNIQUE INDEX shifts_one_open_per_store_idx ON atlas.shifts (store_id) WHERE status = 'open';

ALTER TABLE atlas.shifts ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.shifts FORCE ROW LEVEL SECURITY;
CREATE POLICY shifts_isolation ON atlas.shifts
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === Customers

CREATE TABLE atlas.customers (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    full_name TEXT NOT NULL,
    phone TEXT,
    email TEXT,
    customer_group_id UUID REFERENCES atlas.customer_groups(id),
    store_credit_amount_minor BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX customers_tenant_idx ON atlas.customers (tenant_id);

ALTER TABLE atlas.customers ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.customers FORCE ROW LEVEL SECURITY;
CREATE POLICY customers_isolation ON atlas.customers
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === Sales

CREATE TYPE atlas.sale_status AS ENUM ('held', 'completed', 'voided');

CREATE TABLE atlas.sales (
    -- Client-generated (UUIDv7), not app-generated - see the header
    -- comment. This is the one table in the schema where that's true.
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    shift_id UUID NOT NULL REFERENCES atlas.shifts(id),
    customer_id UUID REFERENCES atlas.customers(id),
    cashier_user_id UUID NOT NULL REFERENCES atlas.users(id),

    -- Mandatory per docs/Atlas/README.md's offline requirements.
    device_id TEXT NOT NULL,

    status atlas.sale_status NOT NULL DEFAULT 'completed',
    subtotal_amount_minor BIGINT NOT NULL,
    discount_amount_minor BIGINT NOT NULL DEFAULT 0,
    tax_amount_minor BIGINT NOT NULL DEFAULT 0,
    total_amount_minor BIGINT NOT NULL,

    -- Reason the server actually accepted this sale after sync, distinct
    -- from client wall-clock time, which is untrustworthy for ordering.
    synced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    voided_at TIMESTAMPTZ,
    voided_by_user_id UUID REFERENCES atlas.users(id)
);

CREATE INDEX sales_store_idx ON atlas.sales (store_id);
CREATE INDEX sales_shift_idx ON atlas.sales (shift_id);
CREATE INDEX sales_tenant_idx ON atlas.sales (tenant_id);

ALTER TABLE atlas.sales ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.sales FORCE ROW LEVEL SECURITY;
CREATE POLICY sales_isolation ON atlas.sales
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.sale_lines (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    sale_id UUID NOT NULL REFERENCES atlas.sales(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id),
    stock_item_id UUID REFERENCES atlas.stock_items(id),
    unit_id UUID NOT NULL REFERENCES atlas.units_of_measure(id),

    quantity NUMERIC NOT NULL CHECK (quantity > 0),
    unit_price_amount_minor BIGINT NOT NULL,
    line_total_amount_minor BIGINT NOT NULL
);

CREATE INDEX sale_lines_sale_idx ON atlas.sale_lines (sale_id);

ALTER TABLE atlas.sale_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.sale_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY sale_lines_isolation ON atlas.sale_lines
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === Returns

CREATE TABLE atlas.sale_returns (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    sale_id UUID NOT NULL REFERENCES atlas.sales(id),
    sale_line_id UUID NOT NULL REFERENCES atlas.sale_lines(id),
    quantity NUMERIC NOT NULL CHECK (quantity > 0),
    refund_amount_minor BIGINT NOT NULL,
    reason TEXT NOT NULL,
    processed_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX sale_returns_sale_idx ON atlas.sale_returns (sale_id);

ALTER TABLE atlas.sale_returns ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.sale_returns FORCE ROW LEVEL SECURITY;
CREATE POLICY sale_returns_isolation ON atlas.sale_returns
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
