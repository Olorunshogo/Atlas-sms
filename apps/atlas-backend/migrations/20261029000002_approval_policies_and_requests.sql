/*
 * FR-09 and FR-29 approval policies and requests, FR-30 "approval policies" as managed configuration.
 * The per-store threshold columns move here, so every limit lives in one table.
 */

-- === Policies

CREATE TYPE atlas.approval_operation AS ENUM ('price_change', 'stock_adjustment', 'write_off', 'refund', 'expense');

/*
 * One limit per operation, business-wide (`store_id` NULL) or for one store, which wins over the business-wide one.
 * Each operation uses exactly one limit column: price changes a percentage of the old price, stock changes a
 * quantity, refunds and expenses an amount.
 */
CREATE TABLE atlas.approval_policies (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID REFERENCES atlas.stores(id) ON DELETE CASCADE,
    operation atlas.approval_operation NOT NULL,
    limit_amount_minor BIGINT CHECK (limit_amount_minor > 0),
    limit_quantity NUMERIC CHECK (limit_quantity > 0),
    limit_basis_points INT CHECK (limit_basis_points > 0),
    updated_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT approval_policy_has_its_limit CHECK (
        CASE operation
            WHEN 'price_change' THEN limit_basis_points IS NOT NULL AND limit_amount_minor IS NULL AND limit_quantity IS NULL
            WHEN 'stock_adjustment' THEN limit_quantity IS NOT NULL AND limit_amount_minor IS NULL AND limit_basis_points IS NULL
            WHEN 'write_off' THEN limit_quantity IS NOT NULL AND limit_amount_minor IS NULL AND limit_basis_points IS NULL
            ELSE limit_amount_minor IS NOT NULL AND limit_quantity IS NULL AND limit_basis_points IS NULL
        END
    )
);

CREATE UNIQUE INDEX approval_policies_scope_idx ON atlas.approval_policies (tenant_id, store_id, operation) NULLS NOT DISTINCT;

ALTER TABLE atlas.approval_policies ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.approval_policies FORCE ROW LEVEL SECURITY;
CREATE POLICY approval_policies_isolation ON atlas.approval_policies
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- The business creator stands in for whoever set the old store columns
INSERT INTO atlas.approval_policies (id, tenant_id, store_id, operation, limit_quantity, updated_by_user_id)
SELECT gen_random_uuid(), s.tenant_id, s.id, 'stock_adjustment', s.adjustment_approval_quantity, t.created_by_user_id
FROM atlas.stores s JOIN atlas.tenants t ON t.id = s.tenant_id
WHERE s.adjustment_approval_quantity IS NOT NULL;

INSERT INTO atlas.approval_policies (id, tenant_id, store_id, operation, limit_quantity, updated_by_user_id)
SELECT gen_random_uuid(), s.tenant_id, s.id, 'write_off', s.write_off_approval_quantity, t.created_by_user_id
FROM atlas.stores s JOIN atlas.tenants t ON t.id = s.tenant_id
WHERE s.write_off_approval_quantity IS NOT NULL;

INSERT INTO atlas.approval_policies (id, tenant_id, store_id, operation, limit_amount_minor, updated_by_user_id)
SELECT gen_random_uuid(), s.tenant_id, s.id, 'refund', s.refund_approval_threshold_minor, t.created_by_user_id
FROM atlas.stores s JOIN atlas.tenants t ON t.id = s.tenant_id
WHERE s.refund_approval_threshold_minor IS NOT NULL;

ALTER TABLE atlas.stores
    DROP COLUMN adjustment_approval_quantity,
    DROP COLUMN write_off_approval_quantity,
    DROP COLUMN refund_approval_threshold_minor;

-- === Requests

CREATE TYPE atlas.approval_request_status AS ENUM ('pending', 'approved', 'rejected', 'cancelled');

/*
 * A change held back by a policy: the request body as it was sent, replayed when someone approves it.
 * Refunds and expenses don't come here - a return and an expense are already their own request.
 */
CREATE TABLE atlas.approval_requests (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID REFERENCES atlas.stores(id) ON DELETE CASCADE,
    operation atlas.approval_operation NOT NULL CHECK (operation IN ('price_change', 'stock_adjustment', 'write_off')),
    status atlas.approval_request_status NOT NULL DEFAULT 'pending',
    subject_id UUID NOT NULL,
    payload JSONB NOT NULL,
    summary TEXT NOT NULL,
    requested_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_by_user_id UUID REFERENCES atlas.users(id),
    decided_at TIMESTAMPTZ,
    decision_note TEXT,
    result_entity_type TEXT,
    result_id UUID
);

CREATE INDEX approval_requests_pending_idx ON atlas.approval_requests (tenant_id, requested_at DESC) WHERE status = 'pending';
CREATE INDEX approval_requests_store_idx ON atlas.approval_requests (store_id, requested_at DESC);

ALTER TABLE atlas.approval_requests ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.approval_requests FORCE ROW LEVEL SECURITY;
CREATE POLICY approval_requests_isolation ON atlas.approval_requests
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

UPDATE atlas.roles
SET permissions = permissions || ARRAY['price.approve']::atlas.permission[]
WHERE is_system AND name IN ('owner', 'admin', 'platform_admin');

-- === Store credit spent at checkout

ALTER TABLE atlas.customer_credit_movements ADD COLUMN payment_id UUID REFERENCES atlas.payments(id);
