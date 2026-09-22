/*
 * OTP storage backing signup email verification. Not tenant-scoped - keyed
 * by user_id, same reachability model as atlas.users itself, so no RLS
 * policy here.
 */
CREATE TABLE atlas.email_otps (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES atlas.users(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    purpose TEXT NOT NULL DEFAULT 'signup_verification',
    attempts INT NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX email_otps_user_idx ON atlas.email_otps (user_id);
