-- CivicSort: Disputed Classification Support
-- Migration 008

CREATE TYPE dispute_status AS ENUM (
    'open',
    'under_review',
    'resolved',
    'dismissed'
);

CREATE TABLE classification_disputes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kb_entry_id     UUID NOT NULL REFERENCES kb_entries(id) ON DELETE CASCADE,
    disputed_by     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason          TEXT NOT NULL,
    proposed_category VARCHAR(255),
    proposed_instructions TEXT,
    status          dispute_status NOT NULL DEFAULT 'open',
    resolution_notes TEXT,
    resolved_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at     TIMESTAMPTZ
);

CREATE INDEX idx_disputes_entry ON classification_disputes(kb_entry_id);
CREATE INDEX idx_disputes_status ON classification_disputes(status);
CREATE INDEX idx_disputes_user ON classification_disputes(disputed_by);
