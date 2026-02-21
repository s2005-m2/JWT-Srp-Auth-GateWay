CREATE TABLE captchas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    text VARCHAR(10) NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '60 seconds'
);
CREATE INDEX idx_captchas_expires_at ON captchas (expires_at);
