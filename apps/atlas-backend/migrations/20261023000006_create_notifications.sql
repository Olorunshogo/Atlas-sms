/*
 * FR-26 (Notifications and Operational Alerts), FR-13 (low-stock alerts).
 * One row per event, fanned out to the recipients authorized for it at
 * write time.
 */
CREATE TYPE atlas.notification_kind AS ENUM (
    'low_stock',
    'transfer_requested', 'transfer_approved', 'transfer_rejected', 'transfer_dispatched', 'transfer_received',
    'stock_count_submitted',
    'purchase_order_submitted',
    'expense_submitted', 'expense_approved', 'expense_rejected'
);

CREATE TYPE atlas.notification_status AS ENUM ('unread', 'read');

CREATE TABLE atlas.notifications (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID REFERENCES atlas.stores(id) ON DELETE CASCADE,
    kind atlas.notification_kind NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    -- Open alerts sharing a key are one condition, e.g. `low_stock:{store}:{product}`
    dedupe_key TEXT,
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX notifications_open_dedupe_idx ON atlas.notifications (tenant_id, dedupe_key)
    WHERE dedupe_key IS NOT NULL AND resolved_at IS NULL;

ALTER TABLE atlas.notifications ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.notifications FORCE ROW LEVEL SECURITY;
CREATE POLICY notifications_isolation ON atlas.notifications
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.notification_recipients (
    notification_id UUID NOT NULL REFERENCES atlas.notifications(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES atlas.users(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    status atlas.notification_status NOT NULL DEFAULT 'unread',
    read_at TIMESTAMPTZ,

    PRIMARY KEY (notification_id, user_id)
);

CREATE INDEX notification_recipients_inbox_idx ON atlas.notification_recipients (user_id, status);

ALTER TABLE atlas.notification_recipients ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.notification_recipients FORCE ROW LEVEL SECURITY;
CREATE POLICY notification_recipients_isolation ON atlas.notification_recipients
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
