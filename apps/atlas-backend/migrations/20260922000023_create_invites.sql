/*
 * Backs the invite flow (docs/atlas/auth-flow.md #5) for a role grant to
 * an email with no existing account yet. An existing user never needs a
 * row here - they get a role_grants row directly (see
 * roles::service::invite_existing_user), since there's nothing left to
 * verify: they already proved control of their email at their own signup.
 *
 * Not tenant-scoped via RLS: reachable only by token (anyone with the
 * emailed link), same reachability model as an OTP, and the accept
 * handler runs through AdminTx since no session exists yet.
 */
CREATE TABLE atlas.invites (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID REFERENCES atlas.stores(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES atlas.roles(id),
    email TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    invited_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX invites_email_idx ON atlas.invites (email);
