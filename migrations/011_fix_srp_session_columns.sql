-- SRP 2048-bit values = 256 bytes = 512 hex chars; split concatenated data into separate columns
ALTER TABLE srp_sessions ADD COLUMN verifier_cache VARCHAR(512);
