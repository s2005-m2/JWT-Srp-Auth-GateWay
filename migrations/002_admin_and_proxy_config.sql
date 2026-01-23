-- Admin users table (separate from regular users)
CREATE TABLE IF NOT EXISTS admins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(100) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One-time admin registration tokens
CREATE TABLE IF NOT EXISTS admin_registration_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    used_by UUID REFERENCES admins(id),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Proxy upstream configurations
CREATE TABLE IF NOT EXISTS proxy_upstreams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    address VARCHAR(255) NOT NULL,
    health_check_path VARCHAR(255) DEFAULT '/health',
    health_check_interval_secs INTEGER DEFAULT 30,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Proxy route rules
CREATE TABLE IF NOT EXISTS proxy_routes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    path_prefix VARCHAR(255) NOT NULL UNIQUE,
    upstream_id UUID NOT NULL REFERENCES proxy_upstreams(id) ON DELETE CASCADE,
    strip_prefix BOOLEAN NOT NULL DEFAULT FALSE,
    require_auth BOOLEAN NOT NULL DEFAULT TRUE,
    priority INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_proxy_routes_priority ON proxy_routes(priority DESC);

-- Rate limit rules
CREATE TABLE IF NOT EXISTS rate_limit_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    path_pattern VARCHAR(255) NOT NULL,
    limit_by VARCHAR(20) NOT NULL DEFAULT 'ip',
    max_requests INTEGER NOT NULL,
    window_secs INTEGER NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Dynamic JWT configuration (stored in DB for hot reload)
CREATE TABLE IF NOT EXISTS jwt_config (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    access_token_ttl_secs INTEGER NOT NULL DEFAULT 86400,
    refresh_token_ttl_secs INTEGER NOT NULL DEFAULT 604800,
    auto_refresh_threshold_secs INTEGER NOT NULL DEFAULT 3600,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default JWT config
INSERT INTO jwt_config (id, access_token_ttl_secs, refresh_token_ttl_secs, auto_refresh_threshold_secs)
VALUES (1, 86400, 604800, 3600)
ON CONFLICT (id) DO NOTHING;

-- Insert default upstream (example)
INSERT INTO proxy_upstreams (name, address, health_check_path, enabled)
VALUES ('default', '127.0.0.1:7000', '/health', TRUE)
ON CONFLICT (name) DO NOTHING;

-- Insert default routes
INSERT INTO proxy_routes (path_prefix, upstream_id, require_auth, priority)
SELECT '/api/', id, TRUE, 100 FROM proxy_upstreams WHERE name = 'default'
ON CONFLICT (path_prefix) DO NOTHING;

INSERT INTO proxy_routes (path_prefix, upstream_id, require_auth, priority)
SELECT '/ws/', id, TRUE, 100 FROM proxy_upstreams WHERE name = 'default'
ON CONFLICT (path_prefix) DO NOTHING;
