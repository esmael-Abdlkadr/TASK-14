-- CivicSort: Waste-Sorting Knowledge Base Schema
-- Migration 002: Knowledge base with versioning, fuzzy search, and image dedup

-- ============================================================
-- Knowledge base categories
-- ============================================================
CREATE TABLE kb_categories (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    parent_id   UUID REFERENCES kb_categories(id) ON DELETE SET NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_kb_categories_parent ON kb_categories(parent_id);

-- ============================================================
-- Knowledge base entries (current head pointer)
-- ============================================================
CREATE TABLE kb_entries (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_name       VARCHAR(500) NOT NULL,
    category_id     UUID REFERENCES kb_categories(id) ON DELETE SET NULL,
    current_version INTEGER NOT NULL DEFAULT 1,
    region          VARCHAR(255) NOT NULL DEFAULT 'default',
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_kb_entries_item_name ON kb_entries(item_name);
CREATE INDEX idx_kb_entries_region ON kb_entries(region);
CREATE INDEX idx_kb_entries_category ON kb_entries(category_id);
CREATE INDEX idx_kb_entries_active ON kb_entries(is_active) WHERE is_active = TRUE;

-- Full-text search index on item_name
CREATE INDEX idx_kb_entries_fts ON kb_entries
    USING GIN (to_tsvector('english', item_name));

-- ============================================================
-- Knowledge base entry versions (immutable version history)
-- ============================================================
CREATE TABLE kb_entry_versions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_id            UUID NOT NULL REFERENCES kb_entries(id) ON DELETE CASCADE,
    version_number      INTEGER NOT NULL,
    item_name           VARCHAR(500) NOT NULL,
    disposal_category   VARCHAR(255) NOT NULL,
    disposal_instructions TEXT NOT NULL,
    special_handling    TEXT,
    contamination_notes TEXT,
    region              VARCHAR(255) NOT NULL,
    rule_source         VARCHAR(500),
    effective_date      DATE NOT NULL DEFAULT CURRENT_DATE,
    change_summary      TEXT,
    created_by          UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(entry_id, version_number)
);

CREATE INDEX idx_kb_versions_entry ON kb_entry_versions(entry_id);
CREATE INDEX idx_kb_versions_effective ON kb_entry_versions(effective_date);

-- ============================================================
-- Aliases and common misspellings for fuzzy matching
-- ============================================================
CREATE TABLE kb_aliases (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_id    UUID NOT NULL REFERENCES kb_entries(id) ON DELETE CASCADE,
    alias       VARCHAR(500) NOT NULL,
    alias_type  VARCHAR(50) NOT NULL DEFAULT 'alias',
    -- alias_type: 'alias', 'misspelling', 'abbreviation', 'colloquial'
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_kb_aliases_entry ON kb_aliases(entry_id);
CREATE INDEX idx_kb_aliases_alias ON kb_aliases(alias);
CREATE INDEX idx_kb_aliases_trigram ON kb_aliases USING GIN (alias gin_trgm_ops);

-- ============================================================
-- Reference images with fingerprint deduplication
-- ============================================================
CREATE TABLE kb_images (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_name       VARCHAR(500) NOT NULL,
    file_path       TEXT NOT NULL,
    file_size       BIGINT NOT NULL,
    mime_type       VARCHAR(100) NOT NULL,
    sha256_hash     VARCHAR(64) NOT NULL UNIQUE,
    width           INTEGER,
    height          INTEGER,
    uploaded_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_kb_images_hash ON kb_images(sha256_hash);

-- ============================================================
-- Junction: entry versions <-> images (many-to-many)
-- ============================================================
CREATE TABLE kb_entry_version_images (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id      UUID NOT NULL REFERENCES kb_entry_versions(id) ON DELETE CASCADE,
    image_id        UUID NOT NULL REFERENCES kb_images(id) ON DELETE CASCADE,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    caption         TEXT,
    UNIQUE(version_id, image_id)
);

CREATE INDEX idx_kb_version_images_version ON kb_entry_version_images(version_id);
CREATE INDEX idx_kb_version_images_image ON kb_entry_version_images(image_id);

-- ============================================================
-- Configurable search weights for fuzzy matching
-- ============================================================
CREATE TABLE kb_search_config (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name_exact_weight       REAL NOT NULL DEFAULT 100.0,
    name_prefix_weight      REAL NOT NULL DEFAULT 80.0,
    name_fuzzy_weight       REAL NOT NULL DEFAULT 60.0,
    alias_exact_weight      REAL NOT NULL DEFAULT 90.0,
    alias_fuzzy_weight      REAL NOT NULL DEFAULT 50.0,
    category_boost          REAL NOT NULL DEFAULT 10.0,
    region_boost            REAL NOT NULL DEFAULT 15.0,
    recency_boost           REAL NOT NULL DEFAULT 5.0,
    fuzzy_threshold         REAL NOT NULL DEFAULT 0.3,
    max_results             INTEGER NOT NULL DEFAULT 20,
    updated_by              UUID REFERENCES users(id) ON DELETE SET NULL,
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default search configuration
INSERT INTO kb_search_config (
    name_exact_weight, name_prefix_weight, name_fuzzy_weight,
    alias_exact_weight, alias_fuzzy_weight,
    category_boost, region_boost, recency_boost,
    fuzzy_threshold, max_results
) VALUES (
    100.0, 80.0, 60.0,
    90.0, 50.0,
    10.0, 15.0, 5.0,
    0.3, 20
);

-- ============================================================
-- Enable pg_trgm extension for fuzzy/trigram matching
-- ============================================================
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Trigram indexes for fuzzy search on item names
CREATE INDEX idx_kb_entries_trigram ON kb_entries USING GIN (item_name gin_trgm_ops);
