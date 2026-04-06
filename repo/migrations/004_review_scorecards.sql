-- CivicSort: Review Workspace & Scorecards Schema
-- Migration 004: Scorecards, review assignments, scoring, consistency, COI

-- ============================================================
-- Enums
-- ============================================================

CREATE TYPE review_target_type AS ENUM (
    'inspection_submission',
    'disputed_classification'
);

CREATE TYPE assignment_method AS ENUM (
    'automatic',
    'manual'
);

CREATE TYPE review_assignment_status AS ENUM (
    'pending',
    'in_progress',
    'completed',
    'recused',
    'reassigned'
);

CREATE TYPE review_status AS ENUM (
    'draft',
    'submitted',
    'finalized'
);

CREATE TYPE consistency_severity AS ENUM (
    'warning',
    'error'
);

-- ============================================================
-- Scorecards (configurable scoring templates)
-- ============================================================
CREATE TABLE scorecards (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(500) NOT NULL,
    description     TEXT,
    target_type     review_target_type NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    passing_score   REAL,                -- minimum weighted score to pass
    created_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_scorecards_target ON scorecards(target_type);
CREATE INDEX idx_scorecards_active ON scorecards(is_active) WHERE is_active = TRUE;

-- ============================================================
-- Scorecard dimensions (weighted scoring criteria)
-- ============================================================
CREATE TABLE scorecard_dimensions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scorecard_id    UUID NOT NULL REFERENCES scorecards(id) ON DELETE CASCADE,
    name            VARCHAR(500) NOT NULL,
    description     TEXT,
    weight          REAL NOT NULL DEFAULT 1.0,
    sort_order      INTEGER NOT NULL DEFAULT 0,

    -- Rating levels (JSON array of {value, label} objects)
    rating_levels   JSONB NOT NULL DEFAULT '[
        {"value": 1, "label": "Poor"},
        {"value": 2, "label": "Below Average"},
        {"value": 3, "label": "Average"},
        {"value": 4, "label": "Good"},
        {"value": 5, "label": "Excellent"}
    ]'::jsonb,

    -- Require a comment for this dimension?
    comment_required BOOLEAN NOT NULL DEFAULT FALSE,
    -- Require comment if rating <= threshold
    comment_required_below INTEGER,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_scorecard_dims_card ON scorecard_dimensions(scorecard_id);

-- ============================================================
-- Consistency rules (detect contradictory ratings)
-- ============================================================
CREATE TABLE consistency_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scorecard_id    UUID NOT NULL REFERENCES scorecards(id) ON DELETE CASCADE,
    name            VARCHAR(500) NOT NULL,
    description     TEXT,
    severity        consistency_severity NOT NULL DEFAULT 'warning',

    -- Rule definition: if dimension_a gets rating in range_a,
    -- then dimension_b should be in range_b. Violation = flag.
    dimension_a_id  UUID NOT NULL REFERENCES scorecard_dimensions(id) ON DELETE CASCADE,
    range_a_min     INTEGER NOT NULL,
    range_a_max     INTEGER NOT NULL,
    dimension_b_id  UUID NOT NULL REFERENCES scorecard_dimensions(id) ON DELETE CASCADE,
    range_b_min     INTEGER NOT NULL,
    range_b_max     INTEGER NOT NULL,

    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_consistency_rules_card ON consistency_rules(scorecard_id);

-- ============================================================
-- Review assignments (links reviewer to reviewable item)
-- ============================================================
CREATE TABLE review_assignments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reviewer_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type     review_target_type NOT NULL,
    target_id       UUID NOT NULL,          -- submission ID or classification dispute ID
    scorecard_id    UUID NOT NULL REFERENCES scorecards(id) ON DELETE CASCADE,

    method          assignment_method NOT NULL DEFAULT 'automatic',
    status          review_assignment_status NOT NULL DEFAULT 'pending',
    is_blind        BOOLEAN NOT NULL DEFAULT FALSE,

    -- Recusal/reassignment tracking
    recused_at      TIMESTAMPTZ,
    recusal_reason  TEXT,
    reassigned_from UUID REFERENCES review_assignments(id) ON DELETE SET NULL,

    assigned_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    assigned_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    due_date        DATE,
    completed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_review_assign_reviewer ON review_assignments(reviewer_id);
CREATE INDEX idx_review_assign_target ON review_assignments(target_type, target_id);
CREATE INDEX idx_review_assign_status ON review_assignments(status);
CREATE INDEX idx_review_assign_pending ON review_assignments(reviewer_id, status)
    WHERE status IN ('pending', 'in_progress');

-- ============================================================
-- Reviews (the reviewer's scored assessment)
-- ============================================================
CREATE TABLE reviews (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    assignment_id   UUID NOT NULL REFERENCES review_assignments(id) ON DELETE CASCADE,
    reviewer_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scorecard_id    UUID NOT NULL REFERENCES scorecards(id) ON DELETE CASCADE,
    target_type     review_target_type NOT NULL,
    target_id       UUID NOT NULL,

    status          review_status NOT NULL DEFAULT 'draft',
    overall_score   REAL,                   -- computed weighted score
    overall_comment TEXT,
    recommendation  VARCHAR(50),            -- 'approve', 'reject', 'revise'

    submitted_at    TIMESTAMPTZ,
    finalized_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reviews_assignment ON reviews(assignment_id);
CREATE INDEX idx_reviews_reviewer ON reviews(reviewer_id);
CREATE INDEX idx_reviews_target ON reviews(target_type, target_id);
CREATE INDEX idx_reviews_status ON reviews(status);

-- ============================================================
-- Review scores (per-dimension ratings)
-- ============================================================
CREATE TABLE review_scores (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    review_id       UUID NOT NULL REFERENCES reviews(id) ON DELETE CASCADE,
    dimension_id    UUID NOT NULL REFERENCES scorecard_dimensions(id) ON DELETE CASCADE,
    rating          INTEGER NOT NULL,
    comment         TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(review_id, dimension_id)
);

CREATE INDEX idx_review_scores_review ON review_scores(review_id);

-- ============================================================
-- Consistency check results (flagged contradictions)
-- ============================================================
CREATE TABLE consistency_check_results (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    review_id       UUID NOT NULL REFERENCES reviews(id) ON DELETE CASCADE,
    rule_id         UUID NOT NULL REFERENCES consistency_rules(id) ON DELETE CASCADE,
    severity        consistency_severity NOT NULL,
    message         TEXT NOT NULL,
    acknowledged    BOOLEAN NOT NULL DEFAULT FALSE,
    acknowledged_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_consistency_results_review ON consistency_check_results(review_id);

-- ============================================================
-- Conflict of interest declarations
-- ============================================================
CREATE TABLE conflict_of_interest (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reviewer_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conflict_type   VARCHAR(100) NOT NULL,
    -- conflict_type: 'department', 'previous_involvement', 'declared'
    target_user_id  UUID REFERENCES users(id) ON DELETE CASCADE,
    department      VARCHAR(255),
    description     TEXT,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    declared_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    declared_by     UUID REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_coi_reviewer ON conflict_of_interest(reviewer_id);
CREATE INDEX idx_coi_target_user ON conflict_of_interest(target_user_id);
CREATE INDEX idx_coi_active ON conflict_of_interest(is_active) WHERE is_active = TRUE;

-- ============================================================
-- Reviewer departments (for COI matching)
-- ============================================================
CREATE TABLE reviewer_departments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    department      VARCHAR(255) NOT NULL,
    is_primary      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, department)
);

CREATE INDEX idx_reviewer_depts_user ON reviewer_departments(user_id);
CREATE INDEX idx_reviewer_depts_dept ON reviewer_departments(department);
