/*
 * Returns and refunds (FR-18). A return is a header plus lines against the
 * original sale's lines; each line records the item's condition and
 * whether it goes back on the shelf. Refunds become sale-level records,
 * linked to the return that caused them and to the payment they went back
 * through (if any), so nothing can be refunded twice or beyond what was
 * paid.
 */

-- The original line-level table was never written by any code. Refuse to
-- drop it if that has somehow changed, rather than lose data silently.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM atlas.sale_returns) THEN
        RAISE EXCEPTION 'atlas.sale_returns has rows; migrate them before dropping the table';
    END IF;
END
$$;
DROP TABLE atlas.sale_returns;

CREATE TYPE atlas.return_status AS ENUM ('pending_approval', 'completed', 'rejected');
CREATE TYPE atlas.item_condition AS ENUM ('resellable', 'opened', 'damaged', 'defective', 'expired');

/*
 * return_window_days counts whole days in the store's timezone after the
 * sale completed: 7 means returns are accepted until the end of the 7th
 * local day. Outside it a return needs approval. A refund at or above
 * refund_approval_threshold_minor needs approval too (NULL = no amount
 * limit). Only conditions listed in restockable_conditions put stock back.
 */
ALTER TABLE atlas.stores
    ADD COLUMN return_window_days INT NOT NULL DEFAULT 7 CHECK (return_window_days >= 0),
    ADD COLUMN refund_approval_threshold_minor BIGINT CHECK (refund_approval_threshold_minor > 0),
    ADD COLUMN restockable_conditions atlas.item_condition[] NOT NULL DEFAULT '{resellable}';

CREATE TABLE atlas.returns (
    -- Client-generated, like a sale's id, so a retried return can't be
    -- recorded twice.
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id),
    sale_id UUID NOT NULL REFERENCES atlas.sales(id),
    return_number TEXT,
    status atlas.return_status NOT NULL,
    reason_code TEXT NOT NULL,
    notes TEXT,
    -- 'original' (back through the sale's own payments), 'store_credit',
    -- or 'payment_method' (a named method, e.g. cash out for a card sale).
    refund_method TEXT NOT NULL CHECK (refund_method IN ('original', 'store_credit', 'payment_method')),
    refund_payment_method_id UUID REFERENCES atlas.payment_methods(id),
    refund_reference TEXT,
    outside_window BOOLEAN NOT NULL DEFAULT false,
    refund_total_minor BIGINT NOT NULL CHECK (refund_total_minor >= 0),
    requested_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    approved_by_user_id UUID REFERENCES atlas.users(id),
    approved_at TIMESTAMPTZ,
    rejected_by_user_id UUID REFERENCES atlas.users(id),
    rejection_reason TEXT,
    completed_at TIMESTAMPTZ,
    device_id TEXT,
    request_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX returns_sale_idx ON atlas.returns (sale_id);
CREATE INDEX returns_store_created_idx ON atlas.returns (store_id, created_at DESC);
CREATE UNIQUE INDEX returns_number_idx ON atlas.returns (tenant_id, return_number) WHERE return_number IS NOT NULL;

ALTER TABLE atlas.returns ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.returns FORCE ROW LEVEL SECURITY;
CREATE POLICY returns_isolation ON atlas.returns
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.return_lines (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    return_id UUID NOT NULL REFERENCES atlas.returns(id) ON DELETE CASCADE,
    sale_line_id UUID NOT NULL REFERENCES atlas.sale_lines(id),
    quantity NUMERIC NOT NULL CHECK (quantity > 0),
    base_quantity NUMERIC NOT NULL CHECK (base_quantity > 0),
    condition atlas.item_condition NOT NULL,
    restock BOOLEAN NOT NULL,
    refund_amount_minor BIGINT NOT NULL CHECK (refund_amount_minor >= 0),
    tax_refund_minor BIGINT NOT NULL DEFAULT 0,
    -- Set only for restocked lines, once the stock has gone back in.
    stock_movement_id UUID REFERENCES atlas.stock_movements(id)
);

CREATE INDEX return_lines_return_idx ON atlas.return_lines (return_id);
CREATE INDEX return_lines_sale_line_idx ON atlas.return_lines (sale_line_id);

ALTER TABLE atlas.return_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.return_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY return_lines_isolation ON atlas.return_lines
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- Same gap-free per-store counter as receipts, for return numbers.
CREATE TABLE atlas.store_return_sequences (
    store_id UUID PRIMARY KEY REFERENCES atlas.stores(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    next_value BIGINT NOT NULL DEFAULT 1
);

ALTER TABLE atlas.store_return_sequences ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.store_return_sequences FORCE ROW LEVEL SECURITY;
CREATE POLICY store_return_sequences_isolation ON atlas.store_return_sequences
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === Refunds become sale-level

ALTER TABLE atlas.refunds
    ADD COLUMN sale_id UUID REFERENCES atlas.sales(id),
    ADD COLUMN return_id UUID REFERENCES atlas.returns(id),
    ADD COLUMN payment_method_id UUID REFERENCES atlas.payment_methods(id),
    ADD COLUMN reason TEXT,
    ADD COLUMN reference TEXT,
    ADD COLUMN approved_by_user_id UUID REFERENCES atlas.users(id),
    ADD COLUMN approved_at TIMESTAMPTZ,
    ADD COLUMN executed_by_user_id UUID REFERENCES atlas.users(id),
    ADD COLUMN executed_at TIMESTAMPTZ,
    -- The shift open at the store when the money went out, so a cash
    -- refund comes out of the right drawer count.
    ADD COLUMN shift_id UUID REFERENCES atlas.shifts(id),
    ADD CONSTRAINT refunds_amount_positive CHECK (amount_minor > 0);

UPDATE atlas.refunds r
SET sale_id = p.sale_id, payment_method_id = p.payment_method_id
FROM atlas.payments p WHERE p.id = r.payment_id;

ALTER TABLE atlas.refunds
    ALTER COLUMN sale_id SET NOT NULL,
    -- A refund to store credit, or out through a different method, has no
    -- original payment to point at.
    ALTER COLUMN payment_id DROP NOT NULL,
    -- Only the step-up confirm flow uses a token; return refunds are
    -- approved by a person instead.
    ALTER COLUMN confirmation_token_hash DROP NOT NULL,
    ALTER COLUMN confirmation_expires_at DROP NOT NULL;

CREATE INDEX refunds_sale_idx ON atlas.refunds (sale_id);
CREATE INDEX refunds_shift_idx ON atlas.refunds (shift_id);

-- === Store credit ledger

/*
 * Append-only history behind customers.store_credit_amount_minor: both
 * change in the same transaction, so the balance can always be explained
 * row by row.
 */
CREATE TABLE atlas.customer_credit_movements (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    customer_id UUID NOT NULL REFERENCES atlas.customers(id),
    amount_minor BIGINT NOT NULL CHECK (amount_minor <> 0),
    refund_id UUID REFERENCES atlas.refunds(id),
    sale_id UUID REFERENCES atlas.sales(id),
    created_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX customer_credit_movements_customer_idx ON atlas.customer_credit_movements (customer_id);

ALTER TABLE atlas.customer_credit_movements ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.customer_credit_movements FORCE ROW LEVEL SECURITY;
CREATE POLICY customer_credit_movements_isolation ON atlas.customer_credit_movements
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
