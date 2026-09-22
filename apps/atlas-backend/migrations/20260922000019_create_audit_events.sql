/*
 * "Audit: who changed what, when, from which device. In a system handling
 * stock and cash this is not optional and it is not a later feature."
 * (docs/Atlas/README.md). Append-only, same as stock_movements - no
 * updated_at/deleted_at. `before`/`after` are JSONB snapshots rather than
 * typed columns because the audited entity varies (a store, a role grant,
 * a price rule, ...); what's fixed is that every write path that matters
 * produces one row here.
 */
CREATE TABLE atlas.audit_events (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    actor_user_id UUID REFERENCES atlas.users(id),
    device_id TEXT,

    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    action TEXT NOT NULL,

    before JSONB,
    after JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX audit_events_entity_idx ON atlas.audit_events (entity_type, entity_id);
CREATE INDEX audit_events_tenant_idx ON atlas.audit_events (tenant_id);

ALTER TABLE atlas.audit_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.audit_events FORCE ROW LEVEL SECURITY;
CREATE POLICY audit_events_isolation ON atlas.audit_events
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
