/*
 * The refresh token itself is NOT a JWT - an opaque random token, stored
 * hashed, so it can be revoked (logout-everywhere, a compromised token)
 * without waiting for a JWT to naturally expire. Verifying it is a DB
 * lookup, not a decode. Not tenant-scoped, same reachability model as
 * atlas.users.
 */
CREATE TABLE atlas.refresh_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES atlas.users(id) ON DELETE CASCADE,

    -- Carried forward on refresh so a new access token can be issued for
    -- the same tenant session without re-resolving which of a multi-tenant
    -- user's grants was active at login time.
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,

    token_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX refresh_tokens_user_idx ON atlas.refresh_tokens (user_id);
