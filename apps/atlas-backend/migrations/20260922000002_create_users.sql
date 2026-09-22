/*
 * UUIDv7 PKs, app-generated (Uuid::now_v7() in Rust, never a DB default).
 * Sequential IDs would let a tenant enumerate record counts off a public
 * store slug; v7 keeps rows roughly time-ordered for index locality without
 * that leak.
 *
 * One human -> one atlas.users row, ever. Role grants (atlas.role_grants)
 * are what make a user an owner/manager/cashier somewhere; this table holds
 * identity only, not access.
 */
CREATE TABLE atlas.users (
    id UUID PRIMARY KEY,
    full_name TEXT NOT NULL,
    email TEXT NOT NULL,
    phone TEXT,
    password_hash TEXT NOT NULL,
    is_email_verified BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX users_email_active_idx ON atlas.users (email) WHERE deleted_at IS NULL;
