-- CivicSort: Admin Console, KPI Dashboards & Campaign Management
-- Migration 005: Campaigns, tags, KPI snapshots, report configs

-- ============================================================
-- Enums
-- ============================================================

CREATE TYPE campaign_status AS ENUM (
    'draft',
    'scheduled',
    'active',
    'completed',
    'cancelled'
);

CREATE TYPE report_format AS ENUM (
    'csv',
    'pdf'
);

-- ============================================================
-- Campaigns (education campaigns and promos)
-- ============================================================
CREATE TABLE campaigns (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(500) NOT NULL,
    description     TEXT,
    status          campaign_status NOT NULL DEFAULT 'draft',
    start_date      DATE NOT NULL,
    end_date        DATE NOT NULL,
    target_region   VARCHAR(255),
    target_audience TEXT,
    goals           JSONB,
    created_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_campaigns_status ON campaigns(status);
CREATE INDEX idx_campaigns_dates ON campaigns(start_date, end_date);

-- ============================================================
-- Tags (reusable labels for categories, campaigns, KB entries)
-- ============================================================
CREATE TABLE tags (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(255) NOT NULL UNIQUE,
    color           VARCHAR(7),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tags_name ON tags(name);

-- ============================================================
-- Campaign-tag junction
-- ============================================================
CREATE TABLE campaign_tags (
    campaign_id     UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    tag_id          UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (campaign_id, tag_id)
);

-- ============================================================
-- Category-tag junction (extends kb_categories)
-- ============================================================
CREATE TABLE category_tags (
    category_id     UUID NOT NULL REFERENCES kb_categories(id) ON DELETE CASCADE,
    tag_id          UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (category_id, tag_id)
);

-- ============================================================
-- KPI snapshots (periodic metric captures for trending)
-- ============================================================
CREATE TABLE kpi_snapshots (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_name     VARCHAR(255) NOT NULL,
    metric_value    REAL NOT NULL,
    dimensions      JSONB,
    period_start    DATE NOT NULL,
    period_end      DATE NOT NULL,
    captured_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_kpi_snapshots_metric ON kpi_snapshots(metric_name);
CREATE INDEX idx_kpi_snapshots_period ON kpi_snapshots(period_start, period_end);

-- ============================================================
-- Saved report configurations
-- ============================================================
CREATE TABLE report_configs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(500) NOT NULL,
    report_type     VARCHAR(100) NOT NULL,
    -- report_type: 'kpi_summary', 'user_overview', 'task_overview', 'campaign_report', 'audit_report'
    parameters      JSONB NOT NULL DEFAULT '{}'::jsonb,
    format          report_format NOT NULL DEFAULT 'csv',
    created_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_report_configs_type ON report_configs(report_type);
