/*
 * SaaS subscription for Atlas itself, not to be confused with a store's
 * own sales (atlas.sales). Tiered by location count + module access per
 * the PRD's business model - genuinely absent from Klassivox's schema,
 * since schools don't have SaaS tiers the same way.
 */
CREATE TYPE atlas.subscription_tier AS ENUM ('starter', 'growth', 'multi_location');
CREATE TYPE atlas.subscription_status AS ENUM ('trialing', 'active', 'past_due', 'canceled');

CREATE TABLE atlas.subscriptions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL UNIQUE REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    tier atlas.subscription_tier NOT NULL,
    status atlas.subscription_status NOT NULL DEFAULT 'trialing',
    location_cap INT NOT NULL,
    renews_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE atlas.subscriptions ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.subscriptions FORCE ROW LEVEL SECURITY;
CREATE POLICY subscriptions_isolation ON atlas.subscriptions
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

/*
 * Explicit rather than a hardcoded tier->modules table in application
 * code, so a one-off deal ("Growth tier but with loyalty included") is a
 * row, not a code branch. Absence of a row for a module means not
 * entitled - the gate a store's own `store_module_toggles` sits behind.
 */
CREATE TABLE atlas.tenant_entitlements (
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    module atlas.store_module NOT NULL,
    PRIMARY KEY (tenant_id, module)
);

ALTER TABLE atlas.tenant_entitlements ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.tenant_entitlements FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_entitlements_isolation ON atlas.tenant_entitlements
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === Payments (against a sale)

CREATE TYPE atlas.payment_method AS ENUM ('cash', 'card', 'transfer', 'wallet');
CREATE TYPE atlas.payment_status AS ENUM ('pending', 'succeeded', 'failed');

CREATE TABLE atlas.payments (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    sale_id UUID NOT NULL REFERENCES atlas.sales(id),
    method atlas.payment_method NOT NULL,
    amount_minor BIGINT NOT NULL,
    currency CHAR(3) NOT NULL,
    status atlas.payment_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX payments_sale_idx ON atlas.payments (sale_id);

ALTER TABLE atlas.payments ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.payments FORCE ROW LEVEL SECURITY;
CREATE POLICY payments_isolation ON atlas.payments
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

/*
 * Step-up confirmation state machine, same shape as Klassivox's refunds
 * table: a refund is not a single write. The first request marks
 * `awaiting_confirmation` and mints a confirmation token; a second,
 * confirmed request (matching, unexpired token) executes it. A refund
 * above a threshold being a single unconfirmed POST is exactly the kind
 * of mistake or account-takeover action this exists to slow down.
 */
CREATE TYPE atlas.refund_status AS ENUM ('awaiting_confirmation', 'executed', 'expired', 'cancelled');

CREATE TABLE atlas.refunds (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    payment_id UUID NOT NULL REFERENCES atlas.payments(id),
    amount_minor BIGINT NOT NULL,
    status atlas.refund_status NOT NULL DEFAULT 'awaiting_confirmation',
    confirmation_token_hash TEXT NOT NULL,
    confirmation_expires_at TIMESTAMPTZ NOT NULL,
    requested_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX refunds_payment_idx ON atlas.refunds (payment_id);

ALTER TABLE atlas.refunds ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.refunds FORCE ROW LEVEL SECURITY;
CREATE POLICY refunds_isolation ON atlas.refunds
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
