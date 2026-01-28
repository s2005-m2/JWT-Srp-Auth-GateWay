-- API Keys for external integrations
-- 256-bit keys (64 hex chars), stored as SHA256 hash
-- No expiration, permissions stored as JSON array

CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id UUID NOT NULL REFERENCES admins(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    key_hash VARCHAR(64) NOT NULL UNIQUE,
    key_prefix VARCHAR(8) NOT NULL,  -- First 8 chars for identification
    permissions JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_keys_admin_id ON api_keys(admin_id);
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
