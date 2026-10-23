/*
 * Receipts for FR-17. Every completed sale gets a per-store sequential
 * receipt number and an immutable snapshot of everything printed on it, so
 * a reprint years later shows exactly what the customer was handed - even
 * after prices, names or the store's address have changed.
 */

-- What the receipt header prints about the business and the branch.
ALTER TABLE atlas.tenants
    ADD COLUMN legal_name TEXT,
    ADD COLUMN tax_identification_number TEXT;

ALTER TABLE atlas.stores
    ADD COLUMN receipt_prefix TEXT,
    ADD COLUMN phone TEXT,
    ADD COLUMN receipt_header TEXT,
    ADD COLUMN receipt_footer TEXT;

/*
 * One counter row per store, bumped with UPDATE ... RETURNING under its row
 * lock, so numbers are gap-free per store in commit order. A Postgres
 * SEQUENCE would leave gaps on every rolled-back sale, and a receipt book
 * with holes in it is exactly what an auditor asks about.
 */
CREATE TABLE atlas.store_receipt_sequences (
    store_id UUID PRIMARY KEY REFERENCES atlas.stores(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    next_value BIGINT NOT NULL DEFAULT 1
);

ALTER TABLE atlas.store_receipt_sequences ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.store_receipt_sequences FORCE ROW LEVEL SECURITY;
CREATE POLICY store_receipt_sequences_isolation ON atlas.store_receipt_sequences
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

ALTER TABLE atlas.sales ADD COLUMN receipt_number TEXT;
CREATE UNIQUE INDEX sales_receipt_number_idx ON atlas.sales (tenant_id, receipt_number) WHERE receipt_number IS NOT NULL;

-- Written once at completion and never updated.
CREATE TABLE atlas.sale_receipts (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    sale_id UUID NOT NULL UNIQUE REFERENCES atlas.sales(id),
    receipt_number TEXT NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    snapshot JSONB NOT NULL,
    snapshot_version SMALLINT NOT NULL
);

ALTER TABLE atlas.sale_receipts ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.sale_receipts FORCE ROW LEVEL SECURITY;
CREATE POLICY sale_receipts_isolation ON atlas.sale_receipts
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TYPE atlas.receipt_print_kind AS ENUM ('original', 'reprint');

-- Every time a receipt is printed: who, from which till, and whether it was
-- the original or a numbered reprint.
CREATE TABLE atlas.receipt_prints (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    sale_receipt_id UUID NOT NULL REFERENCES atlas.sale_receipts(id),
    printed_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    device_id TEXT,
    kind atlas.receipt_print_kind NOT NULL,
    copy_number INT NOT NULL,
    reason TEXT,
    printed_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (sale_receipt_id, copy_number)
);

ALTER TABLE atlas.receipt_prints ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.receipt_prints FORCE ROW LEVEL SECURITY;
CREATE POLICY receipt_prints_isolation ON atlas.receipt_prints
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
