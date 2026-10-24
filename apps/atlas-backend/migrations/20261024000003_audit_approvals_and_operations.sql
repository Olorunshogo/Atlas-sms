/*
 * FR-27 audit search and immutability, FR-29 approval thresholds, FR-26 and
 * FR-30 notification preferences, delivery outbox and operational issues.
 */

-- === FR-27

ALTER TABLE atlas.audit_events
    ADD COLUMN store_id UUID,
    ADD COLUMN outcome TEXT NOT NULL DEFAULT 'success';

CREATE INDEX audit_events_tenant_time_idx ON atlas.audit_events (tenant_id, created_at DESC);
CREATE INDEX audit_events_actor_idx ON atlas.audit_events (actor_user_id, created_at DESC);

-- Nothing the app does can change or remove an audit row; a tenant's own cascade delete still can
REVOKE UPDATE, DELETE ON atlas.audit_events FROM atlas_app;

CREATE FUNCTION atlas.audit_events_are_append_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'audit events cannot be modified';
END $$;

CREATE TRIGGER audit_events_append_only BEFORE UPDATE ON atlas.audit_events
    FOR EACH ROW EXECUTE FUNCTION atlas.audit_events_are_append_only();

UPDATE atlas.roles
SET permissions = permissions || ARRAY['audit.view']::atlas.permission[]
WHERE is_system AND name IN ('owner', 'admin', 'platform_admin');

-- === FR-29

UPDATE atlas.roles
SET permissions = permissions || ARRAY['stock.approve']::atlas.permission[]
WHERE is_system AND name IN ('owner', 'admin', 'store_manager', 'platform_admin');

/*
 * NULL threshold = no limit. `allow_self_approval` stays on by default so a
 * one-person business can still approve its own counts, orders and expenses.
 */
ALTER TABLE atlas.stores
    ADD COLUMN adjustment_approval_quantity NUMERIC CHECK (adjustment_approval_quantity > 0),
    ADD COLUMN write_off_approval_quantity NUMERIC CHECK (write_off_approval_quantity > 0),
    ADD COLUMN allow_self_approval BOOLEAN NOT NULL DEFAULT true;

-- === FR-25 / FR-30

-- The cost a line was sold at, per base unit, so profit stays explainable after cost prices change
ALTER TABLE atlas.sale_lines ADD COLUMN unit_cost_amount_minor BIGINT;

-- === FR-26 / FR-30

CREATE TABLE atlas.notification_preferences (
    user_id UUID NOT NULL REFERENCES atlas.users(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    kind atlas.notification_kind NOT NULL,
    in_app BOOLEAN NOT NULL DEFAULT true,
    email BOOLEAN NOT NULL DEFAULT false,

    PRIMARY KEY (user_id, tenant_id, kind)
);

ALTER TABLE atlas.notification_preferences ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.notification_preferences FORCE ROW LEVEL SECURITY;
CREATE POLICY notification_preferences_isolation ON atlas.notification_preferences
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TYPE atlas.delivery_status AS ENUM ('pending', 'sent', 'failed');

-- Emails waiting to go out; a worker sends them, retrying, so delivery never blocks the action that raised them
CREATE TABLE atlas.notification_deliveries (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    notification_id UUID NOT NULL REFERENCES atlas.notifications(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES atlas.users(id) ON DELETE CASCADE,
    channel TEXT NOT NULL DEFAULT 'email',
    status atlas.delivery_status NOT NULL DEFAULT 'pending',
    attempts INT NOT NULL DEFAULT 0,
    last_error TEXT,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX notification_deliveries_due_idx ON atlas.notification_deliveries (next_attempt_at) WHERE status = 'pending';

ALTER TABLE atlas.notification_deliveries ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.notification_deliveries FORCE ROW LEVEL SECURITY;
CREATE POLICY notification_deliveries_isolation ON atlas.notification_deliveries
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- Things that went wrong without failing a request: dropped notifications, failed deliveries
CREATE TABLE atlas.operational_issues (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    detail TEXT NOT NULL,
    reference_id UUID,
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX operational_issues_open_idx ON atlas.operational_issues (tenant_id, created_at DESC) WHERE resolved_at IS NULL;

ALTER TABLE atlas.operational_issues ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.operational_issues FORCE ROW LEVEL SECURITY;
CREATE POLICY operational_issues_isolation ON atlas.operational_issues
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
