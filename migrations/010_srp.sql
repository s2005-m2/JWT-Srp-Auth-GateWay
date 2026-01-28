ALTER TABLE users DROP COLUMN IF EXISTS password_hash;

ALTER TABLE users ADD COLUMN srp_salt VARCHAR(64);
ALTER TABLE users ADD COLUMN srp_verifier VARCHAR(512);

CREATE TABLE srp_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_ephemeral_secret VARCHAR(512) NOT NULL,
    client_ephemeral_public VARCHAR(512) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_srp_sessions_expires ON srp_sessions(expires_at);
