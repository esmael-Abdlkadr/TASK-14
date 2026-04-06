-- CivicSort: Bulk Data Management & Deduplication
-- Migration 007: Import/export versioning, fingerprints, duplicates, merges

-- ============================================================
-- Enums
-- ============================================================

CREATE TYPE import_job_status AS ENUM (
    'pending',
    'validating',
    'validated',
    'importing',
    'completed',
    'failed',
    'cancelled'
);

CREATE TYPE import_row_status AS ENUM (
    'pending',
    'valid',
    'duplicate',
    'conflict',
    'imported',
    'skipped',
    'error'
);

CREATE TYPE change_operation AS ENUM (
    'create',
    'update',
    'delete',
    'merge',
    'import',
    'revert'
);

CREATE TYPE merge_request_status AS ENUM (
    'pending',
    'approved',
    'rejected',
    'applied',
    'cancelled'
);

CREATE TYPE duplicate_status AS ENUM (
    'detected',
    'confirmed',
    'dismissed',
    'merged'
);

-- ============================================================
-- Import jobs (bulk CSV/JSON import sessions)
-- ============================================================
CREATE TABLE import_jobs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(500) NOT NULL,
    entity_type     VARCHAR(100) NOT NULL,
    -- entity_type: 'kb_entry', 'user', 'task_template'
    file_name       VARCHAR(500),
    total_rows      INTEGER NOT NULL DEFAULT 0,
    processed_rows  INTEGER NOT NULL DEFAULT 0,
    imported_rows   INTEGER NOT NULL DEFAULT 0,
    duplicate_rows  INTEGER NOT NULL DEFAULT 0,
    error_rows      INTEGER NOT NULL DEFAULT 0,
    status          import_job_status NOT NULL DEFAULT 'pending',
    error_message   TEXT,
    imported_by     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX idx_import_jobs_status ON import_jobs(status);
CREATE INDEX idx_import_jobs_user ON import_jobs(imported_by);

-- ============================================================
-- Import rows (individual records within an import job)
-- ============================================================
CREATE TABLE import_rows (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id          UUID NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
    row_number      INTEGER NOT NULL,
    raw_data        JSONB NOT NULL,
    parsed_data     JSONB,
    status          import_row_status NOT NULL DEFAULT 'pending',
    entity_id       UUID,
    duplicate_of    UUID,
    error_message   TEXT,
    validation_errors JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_import_rows_job ON import_rows(job_id);
CREATE INDEX idx_import_rows_status ON import_rows(status);

-- ============================================================
-- Data change history (who/when/what with reversible history)
-- ============================================================
CREATE TABLE data_changes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type     VARCHAR(100) NOT NULL,
    entity_id       UUID NOT NULL,
    operation       change_operation NOT NULL,
    field_name      VARCHAR(255),
    old_value       JSONB,
    new_value       JSONB,
    change_set_id   UUID,
    import_job_id   UUID REFERENCES import_jobs(id) ON DELETE SET NULL,
    merge_request_id UUID,
    changed_by      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    changed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reverted_at     TIMESTAMPTZ,
    reverted_by     UUID REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_data_changes_entity ON data_changes(entity_type, entity_id);
CREATE INDEX idx_data_changes_changeset ON data_changes(change_set_id);
CREATE INDEX idx_data_changes_user ON data_changes(changed_by);
CREATE INDEX idx_data_changes_time ON data_changes(changed_at);

-- ============================================================
-- Content fingerprints (for near-duplicate detection)
-- ============================================================
CREATE TABLE content_fingerprints (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type     VARCHAR(100) NOT NULL,
    entity_id       UUID NOT NULL,
    fingerprint_type VARCHAR(50) NOT NULL,
    -- fingerprint_type: 'content_hash', 'normalized_url', 'key_fields', 'text_simhash'
    fingerprint     VARCHAR(128) NOT NULL,
    source_text     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fingerprints_entity ON content_fingerprints(entity_type, entity_id);
CREATE INDEX idx_fingerprints_hash ON content_fingerprints(fingerprint_type, fingerprint);
CREATE INDEX idx_fingerprints_lookup ON content_fingerprints(entity_type, fingerprint_type, fingerprint);

-- ============================================================
-- Duplicate flags
-- ============================================================
CREATE TABLE duplicate_flags (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type     VARCHAR(100) NOT NULL,
    source_id       UUID NOT NULL,
    target_id       UUID NOT NULL,
    match_type      VARCHAR(100) NOT NULL,
    -- match_type: 'exact_name', 'content_hash', 'key_fields', 'url_normalized', 'near_duplicate'
    confidence      REAL NOT NULL DEFAULT 1.0,
    status          duplicate_status NOT NULL DEFAULT 'detected',
    details         JSONB,
    detected_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    resolved_at     TIMESTAMPTZ
);

CREATE INDEX idx_dup_flags_entity ON duplicate_flags(entity_type);
CREATE INDEX idx_dup_flags_source ON duplicate_flags(source_id);
CREATE INDEX idx_dup_flags_target ON duplicate_flags(target_id);
CREATE INDEX idx_dup_flags_status ON duplicate_flags(status);

-- ============================================================
-- Merge requests (require manager confirmation)
-- ============================================================
CREATE TABLE merge_requests (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type     VARCHAR(100) NOT NULL,
    source_id       UUID NOT NULL,
    target_id       UUID NOT NULL,
    duplicate_flag_id UUID REFERENCES duplicate_flags(id) ON DELETE SET NULL,
    status          merge_request_status NOT NULL DEFAULT 'pending',
    resolution      JSONB,
    -- resolution: per-field decisions {field: "keep_source"|"keep_target"|"custom", value: ...}
    provenance      JSONB,
    -- provenance: tracks origin of each field value after merge
    requested_by    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reviewed_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    review_notes    TEXT,
    requested_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at     TIMESTAMPTZ,
    applied_at      TIMESTAMPTZ
);

CREATE INDEX idx_merge_requests_status ON merge_requests(status);
CREATE INDEX idx_merge_requests_entity ON merge_requests(entity_type, source_id, target_id);

-- ============================================================
-- Merge conflicts (per-field conflict details)
-- ============================================================
CREATE TABLE merge_conflicts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merge_request_id UUID NOT NULL REFERENCES merge_requests(id) ON DELETE CASCADE,
    field_name      VARCHAR(255) NOT NULL,
    source_value    JSONB,
    target_value    JSONB,
    resolution      VARCHAR(50),
    -- resolution: 'keep_source', 'keep_target', 'custom'
    custom_value    JSONB,
    resolved_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    resolved_at     TIMESTAMPTZ
);

CREATE INDEX idx_merge_conflicts_request ON merge_conflicts(merge_request_id);
