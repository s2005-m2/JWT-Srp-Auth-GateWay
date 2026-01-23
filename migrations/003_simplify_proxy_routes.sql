-- Simplify proxy_routes: remove upstream_id reference, add direct upstream_address
-- Drop old proxy_routes and proxy_upstreams tables, recreate simplified version

-- Drop old tables
DROP TABLE IF EXISTS proxy_routes;
DROP TABLE IF EXISTS proxy_upstreams;

-- Simplified proxy routes (direct path -> upstream mapping)
CREATE TABLE proxy_routes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    path_prefix VARCHAR(255) NOT NULL UNIQUE,
    upstream_address VARCHAR(255) NOT NULL,
    require_auth BOOLEAN NOT NULL DEFAULT TRUE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default routes
INSERT INTO proxy_routes (path_prefix, upstream_address, require_auth)
VALUES 
    ('/api/', '127.0.0.1:7000', TRUE),
    ('/ws/', '127.0.0.1:7000', TRUE);
