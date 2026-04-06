-- CivicSort: Encryption-at-rest for sensitive fields
-- Migration 009

-- Submissions: encrypted_notes is canonical storage for sensitive notes
ALTER TABLE task_submissions ADD COLUMN IF NOT EXISTS encrypted_notes TEXT;

-- Device bindings: encrypted_fingerprint for encrypted storage,
-- fingerprint_hash for deterministic lookup (SHA-256 of raw fingerprint)
ALTER TABLE device_bindings ADD COLUMN IF NOT EXISTS encrypted_fingerprint TEXT;
ALTER TABLE device_bindings ADD COLUMN IF NOT EXISTS fingerprint_hash VARCHAR(64);
CREATE INDEX IF NOT EXISTS idx_device_bindings_fphash
    ON device_bindings(user_id, fingerprint_hash);

-- Audit log: encrypted_details for sensitive audit payloads
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS encrypted_details TEXT;
