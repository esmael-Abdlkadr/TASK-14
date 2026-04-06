-- CivicSort: Authentication, Security & Audit Schema
-- Migration 001: Initial schema

-- User roles enum
CREATE TYPE user_role AS ENUM (
    'field_inspector',
    'reviewer',
    'operations_admin',
    'department_manager'
);

-- Account status enum
CREATE TYPE account_status AS ENUM (
    'active',
    'locked',
    'disabled'
);

-- ============================================================
-- Users table
-- ============================================================
CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username        VARCHAR(255) NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    role            user_role NOT NULL,
    status          account_status NOT NULL DEFAULT 'active',
    locked_until    TIMESTAMPTZ,
    failed_attempts INTEGER NOT NULL DEFAULT 0,
    last_failed_at  TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_status ON users(status);

-- ============================================================
-- Sessions table (idle timeout = 30 minutes)
-- ============================================================
CREATE TABLE sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash      TEXT NOT NULL UNIQUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_active_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL,
    ip_address      VARCHAR(45),
    user_agent      TEXT,
    is_valid        BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_token_hash ON sessions(token_hash);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);

-- ============================================================
-- Login attempts (for lockout tracking and anomaly detection)
-- ============================================================
CREATE TABLE login_attempts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID REFERENCES users(id) ON DELETE SET NULL,
    username        VARCHAR(255) NOT NULL,
    success         BOOLEAN NOT NULL,
    ip_address      VARCHAR(45),
    user_agent      TEXT,
    attempted_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    failure_reason  VARCHAR(255)
);

CREATE INDEX idx_login_attempts_user_id ON login_attempts(user_id);
CREATE INDEX idx_login_attempts_username ON login_attempts(username);
CREATE INDEX idx_login_attempts_attempted_at ON login_attempts(attempted_at);

-- ============================================================
-- Device-account bindings (for step-up verification)
-- ============================================================
CREATE TABLE device_bindings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_fingerprint TEXT NOT NULL,
    device_name     VARCHAR(255),
    bound_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_trusted      BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(user_id, device_fingerprint)
);

CREATE INDEX idx_device_bindings_user_id ON device_bindings(user_id);

-- ============================================================
-- Step-up verification records
-- ============================================================
CREATE TABLE stepup_verifications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id      UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action_type     VARCHAR(100) NOT NULL,
    verified_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_stepup_session ON stepup_verifications(session_id);

-- ============================================================
-- Rate limiting state
-- ============================================================
CREATE TABLE rate_limit_buckets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    bucket_key      VARCHAR(255) NOT NULL,
    request_count   INTEGER NOT NULL DEFAULT 0,
    window_start    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, bucket_key)
);

CREATE INDEX idx_rate_limit_user ON rate_limit_buckets(user_id);

-- ============================================================
-- Encryption keys (application-managed)
-- ============================================================
CREATE TABLE encryption_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key_name        VARCHAR(255) NOT NULL UNIQUE,
    encrypted_key   BYTEA NOT NULL,
    nonce           BYTEA NOT NULL,
    algorithm       VARCHAR(50) NOT NULL DEFAULT 'AES-256-GCM',
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    rotated_at      TIMESTAMPTZ
);

-- ============================================================
-- Immutable audit log
-- ============================================================
CREATE TABLE audit_log (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    user_id         UUID REFERENCES users(id) ON DELETE SET NULL,
    username        VARCHAR(255) NOT NULL,
    role            user_role,
    action          VARCHAR(255) NOT NULL,
    resource_type   VARCHAR(255),
    resource_id     VARCHAR(255),
    details         JSONB,
    ip_address      VARCHAR(45),
    user_agent      TEXT,
    session_id      UUID,
    prev_hash       VARCHAR(128),
    entry_hash      VARCHAR(128) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_log_user_id ON audit_log(user_id);
CREATE INDEX idx_audit_log_action ON audit_log(action);
CREATE INDEX idx_audit_log_created_at ON audit_log(created_at);
CREATE INDEX idx_audit_log_resource ON audit_log(resource_type, resource_id);

-- Make audit_log append-only: deny UPDATE and DELETE via trigger
CREATE OR REPLACE FUNCTION prevent_audit_modification()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'Audit log entries cannot be modified or deleted';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_log_no_update
    BEFORE UPDATE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();

CREATE TRIGGER audit_log_no_delete
    BEFORE DELETE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();
