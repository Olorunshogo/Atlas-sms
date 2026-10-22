/*
 * Server-authoritative checkout (FR-15, FR-16). A sale now stores the full
 * priced snapshot the server computed - names, units, list and charged
 * prices, discounts, tax rate and amount per line - so history and receipts
 * never re-read the live catalogue, plus where it stands on payment.
 */

-- === Sales

CREATE TYPE atlas.sale_payment_status AS ENUM ('unpaid', 'partially_paid', 'paid');

ALTER TABLE atlas.sales
    ADD COLUMN currency CHAR(3),
    ADD COLUMN payment_status atlas.sale_payment_status NOT NULL DEFAULT 'unpaid',
    -- Successful payments applied to the sale (change excluded). Refunds
    -- are tracked separately, so a refunded sale still reads as paid.
    ADD COLUMN amount_paid_minor BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN amount_refunded_minor BIGINT NOT NULL DEFAULT 0,
    -- Cash rounding applied on top of total_amount_minor when paid in cash;
    -- kept apart so line totals never absorb it.
    ADD COLUMN rounding_adjustment_minor BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN cart_discount_amount_minor BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN discount_reason TEXT,
    ADD COLUMN prices_include_tax BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN price_overridden BOOLEAN NOT NULL DEFAULT false,
    -- The device's clock: kept for audit, never trusted for ordering.
    ADD COLUMN client_created_at TIMESTAMPTZ,
    ADD COLUMN completed_at TIMESTAMPTZ,
    -- Fingerprint of the accepted request body: a replay with the same id
    -- and the same body is a no-op, the same id with a different body is
    -- an IDEMPOTENCY_CONFLICT.
    ADD COLUMN request_hash TEXT;

UPDATE atlas.sales s SET currency = st.currency FROM atlas.stores st WHERE st.id = s.store_id;
ALTER TABLE atlas.sales ALTER COLUMN currency SET NOT NULL;

UPDATE atlas.sales SET completed_at = created_at WHERE status = 'completed';

UPDATE atlas.sales s
SET amount_paid_minor = p.paid,
    payment_status = CASE
        WHEN p.paid >= s.total_amount_minor THEN 'paid'::atlas.sale_payment_status
        WHEN p.paid > 0 THEN 'partially_paid'::atlas.sale_payment_status
        ELSE 'unpaid'::atlas.sale_payment_status
    END
FROM (
    SELECT sale_id, SUM(amount_minor) AS paid
    FROM atlas.payments WHERE status = 'succeeded' GROUP BY sale_id
) p
WHERE p.sale_id = s.id;

CREATE INDEX sales_store_created_idx ON atlas.sales (tenant_id, store_id, created_at DESC, id);
CREATE INDEX sales_cashier_created_idx ON atlas.sales (cashier_user_id, created_at DESC);
CREATE INDEX sales_payment_status_idx ON atlas.sales (payment_status);

-- === Sale lines

ALTER TABLE atlas.sale_lines
    ADD COLUMN line_number INT,
    ADD COLUMN product_name TEXT,
    ADD COLUMN sku TEXT,
    ADD COLUMN unit_name TEXT,
    ADD COLUMN base_quantity NUMERIC,
    ADD COLUMN list_unit_price_amount_minor BIGINT,
    ADD COLUMN price_rule_id UUID REFERENCES atlas.price_rules(id) ON DELETE SET NULL,
    ADD COLUMN price_overridden BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN price_override_reason TEXT,
    ADD COLUMN line_discount_amount_minor BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN allocated_cart_discount_amount_minor BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN discount_reason TEXT,
    ADD COLUMN net_amount_minor BIGINT,
    ADD COLUMN tax_rate_id UUID REFERENCES atlas.tax_rates(id),
    ADD COLUMN tax_rate_basis_points INT NOT NULL DEFAULT 0,
    ADD COLUMN tax_amount_minor BIGINT NOT NULL DEFAULT 0;

-- Lines written before the snapshot columns existed: best-effort backfill
-- from the catalogue as it is now.
UPDATE atlas.sale_lines sl
SET product_name = p.name,
    sku = p.sku,
    unit_name = u.name,
    base_quantity = sl.quantity,
    net_amount_minor = sl.line_total_amount_minor,
    line_number = 1
FROM atlas.products p, atlas.units_of_measure u
WHERE p.id = sl.product_id AND u.id = sl.unit_id AND sl.product_name IS NULL;

ALTER TABLE atlas.sale_lines
    ALTER COLUMN line_number SET NOT NULL,
    ALTER COLUMN product_name SET NOT NULL,
    ALTER COLUMN unit_name SET NOT NULL,
    ALTER COLUMN base_quantity SET NOT NULL,
    ALTER COLUMN net_amount_minor SET NOT NULL;

-- Off by default: every sale must be fully paid to complete unless the
-- store explicitly allows pay-later for a named customer.
ALTER TABLE atlas.stores ADD COLUMN allow_pay_later BOOLEAN NOT NULL DEFAULT false;

-- === Payment methods

/*
 * The tenant's configured ways to pay. `kind` is the built-in behaviour
 * (only `cash` gives change, `store_credit` draws on a customer balance);
 * `code`/`name` are what staff see - a tenant can run "Moniepoint POS" and
 * "Opay POS" as two `card`-kind methods and reconcile them separately.
 */
CREATE TABLE atlas.payment_methods (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    kind atlas.payment_method NOT NULL,
    requires_reference BOOLEAN NOT NULL DEFAULT false,
    is_active BOOLEAN NOT NULL DEFAULT true,
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, code)
);

ALTER TABLE atlas.payment_methods ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.payment_methods FORCE ROW LEVEL SECURITY;
CREATE POLICY payment_methods_isolation ON atlas.payment_methods
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- Defaults for tenants that already exist; new tenants get the same set at
-- signup (auth::handlers::signup). gen_random_uuid() here is a one-off
-- backfill - no app runs inside a migration to mint UUIDv7s - every row
-- created afterwards gets its id from Uuid::now_v7() as usual.
INSERT INTO atlas.payment_methods (id, tenant_id, code, name, kind, requires_reference, sort_order)
SELECT gen_random_uuid(), t.id, m.code, m.name, m.kind::atlas.payment_method, m.requires_reference, m.sort_order
FROM atlas.tenants t
CROSS JOIN (VALUES
    ('cash', 'Cash', 'cash', false, 1),
    ('card', 'Card', 'card', false, 2),
    ('transfer', 'Bank transfer', 'transfer', true, 3),
    ('store_credit', 'Store credit', 'store_credit', false, 4)
) AS m(code, name, kind, requires_reference, sort_order);

-- === Payments

ALTER TABLE atlas.payments
    ADD COLUMN payment_method_id UUID REFERENCES atlas.payment_methods(id),
    ADD COLUMN reference TEXT,
    -- Cash only: what the customer handed over; change = tendered - amount.
    ADD COLUMN tendered_amount_minor BIGINT,
    ADD COLUMN change_amount_minor BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN refunded_amount_minor BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN shift_id UUID REFERENCES atlas.shifts(id),
    ADD COLUMN received_by_user_id UUID REFERENCES atlas.users(id),
    ADD COLUMN status_reason TEXT,
    ADD COLUMN processed_at TIMESTAMPTZ,
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD CONSTRAINT payments_amount_positive CHECK (amount_minor > 0),
    ADD CONSTRAINT payments_refund_within_amount CHECK (refunded_amount_minor >= 0 AND refunded_amount_minor <= amount_minor);

UPDATE atlas.payments p
SET payment_method_id = pm.id, shift_id = s.shift_id, processed_at = p.created_at
FROM atlas.payment_methods pm, atlas.sales s
WHERE pm.tenant_id = p.tenant_id AND pm.code = p.method::text AND s.id = p.sale_id;

/*
 * The same transfer or card slip can't be recorded twice while either copy
 * is live. Failed and reversed payments drop out, so a genuine retry with
 * the same reference after a failure still goes through.
 */
CREATE UNIQUE INDEX payments_reference_unique_idx
    ON atlas.payments (tenant_id, payment_method_id, reference)
    WHERE reference IS NOT NULL AND status IN ('pending', 'succeeded');

CREATE INDEX payments_shift_idx ON atlas.payments (shift_id);
