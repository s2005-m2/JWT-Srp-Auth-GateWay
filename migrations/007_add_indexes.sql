-- Add indexes for performance optimization

CREATE INDEX IF NOT EXISTS idx_verification_codes_expires_at 
ON verification_codes(expires_at);

CREATE INDEX IF NOT EXISTS idx_verification_codes_email_type 
ON verification_codes(email, code_type);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires_at 
ON refresh_tokens(expires_at);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id 
ON refresh_tokens(user_id);

CREATE INDEX IF NOT EXISTS idx_users_created_at 
ON users(created_at);
