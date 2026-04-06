-- CivicSort: Device fingerprint hash canonicalization
-- Migration 010: Backfill fingerprint_hash, add unique constraint, phase out plaintext reliance
-- IMPORTANT: pgcrypto MUST be created before any hash function usage.

-- Step 1: Ensure pgcrypto extension exists (required for digest function)
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Step 2: Backfill fingerprint_hash from existing plaintext device_fingerprint
UPDATE device_bindings
SET fingerprint_hash = encode(digest(device_fingerprint, 'sha256'), 'hex')
WHERE fingerprint_hash IS NULL AND device_fingerprint IS NOT NULL;

-- Step 3: Add unique constraint on (user_id, fingerprint_hash) for canonical dedupe
-- Partial index allows NULL fingerprint_hash during transition for any rows that
-- somehow lack both hash and plaintext (should not happen, but safe).
CREATE UNIQUE INDEX IF NOT EXISTS idx_device_bindings_user_fphash_unique
    ON device_bindings(user_id, fingerprint_hash)
    WHERE fingerprint_hash IS NOT NULL;
