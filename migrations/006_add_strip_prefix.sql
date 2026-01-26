-- Add strip_prefix column to proxy_routes
-- This allows stripping path prefix before forwarding to upstream
-- e.g., /api/v1/users -> /v1/users when strip_prefix = '/api'

ALTER TABLE proxy_routes 
ADD COLUMN IF NOT EXISTS strip_prefix VARCHAR(255) DEFAULT NULL;
