/*
 * Deliberately flat: no self-referencing parent_tenant_id. Atlas's stated
 * scale is 1-20 stores per tenant (SystemsRequirements SOM), not the
 * multi-layer diocese/ministry chains that justify Klassivox's ancestor
 * hierarchy. A franchise/holding-company hierarchy (a tenant owning other
 * tenants) is tracked as a possible-later item in docs/v2.md, not built
 * pre-emptively - see the store README's warning against modeling variation
 * that hasn't shown up yet.
 */
CREATE TYPE atlas.tenant_status AS ENUM ('pending', 'active', 'suspended');

CREATE TABLE atlas.tenants (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    status atlas.tenant_status NOT NULL DEFAULT 'pending',
    created_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);
