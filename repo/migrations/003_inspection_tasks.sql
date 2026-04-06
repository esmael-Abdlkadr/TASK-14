-- CivicSort: Inspection Tasks & Scheduling Schema
-- Migration 003: Task templates, scheduling, instances, submissions, reminders

-- ============================================================
-- Enums
-- ============================================================

CREATE TYPE task_cycle AS ENUM (
    'daily',
    'weekly',
    'biweekly',
    'monthly',
    'quarterly',
    'one_time'
);

CREATE TYPE task_instance_status AS ENUM (
    'scheduled',
    'in_progress',
    'submitted',
    'completed',
    'overdue',
    'missed',
    'makeup'
);

CREATE TYPE submission_status AS ENUM (
    'pending_review',
    'approved',
    'rejected',
    'needs_revision'
);

CREATE TYPE reminder_type AS ENUM (
    'upcoming',
    'due_soon',
    'overdue',
    'makeup_deadline',
    'missed_warning'
);

CREATE TYPE reminder_status AS ENUM (
    'unread',
    'read',
    'dismissed'
);

-- ============================================================
-- Task templates (reusable inspection definitions)
-- ============================================================
CREATE TABLE task_templates (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                VARCHAR(500) NOT NULL,
    description         TEXT,
    group_name          VARCHAR(255),

    -- Scheduling rules
    cycle               task_cycle NOT NULL DEFAULT 'weekly',
    time_window_start   TIME NOT NULL DEFAULT '08:00:00',
    time_window_end     TIME NOT NULL DEFAULT '18:00:00',

    -- Fault tolerance
    allowed_misses      INTEGER NOT NULL DEFAULT 1,
    miss_window_days    INTEGER NOT NULL DEFAULT 30,

    -- Make-up rules
    makeup_allowed      BOOLEAN NOT NULL DEFAULT TRUE,
    makeup_deadline_hours INTEGER NOT NULL DEFAULT 48,

    -- Metadata
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    created_by          UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_task_templates_group ON task_templates(group_name);
CREATE INDEX idx_task_templates_active ON task_templates(is_active) WHERE is_active = TRUE;
CREATE INDEX idx_task_templates_cycle ON task_templates(cycle);

-- ============================================================
-- Template subtasks (checklist items within a template)
-- ============================================================
CREATE TABLE template_subtasks (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_id     UUID NOT NULL REFERENCES task_templates(id) ON DELETE CASCADE,
    title           VARCHAR(500) NOT NULL,
    description     TEXT,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    is_required     BOOLEAN NOT NULL DEFAULT TRUE,
    expected_type   VARCHAR(50) NOT NULL DEFAULT 'checkbox',
    -- expected_type: 'checkbox', 'text', 'number', 'photo', 'select'
    options         JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_template_subtasks_template ON template_subtasks(template_id);

-- ============================================================
-- Task schedules (binds template to assignee + date range)
-- ============================================================
CREATE TABLE task_schedules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_id     UUID NOT NULL REFERENCES task_templates(id) ON DELETE CASCADE,
    assigned_to     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    start_date      DATE NOT NULL,
    end_date        DATE,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    notes           TEXT,
    created_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_task_schedules_template ON task_schedules(template_id);
CREATE INDEX idx_task_schedules_assignee ON task_schedules(assigned_to);
CREATE INDEX idx_task_schedules_dates ON task_schedules(start_date, end_date);
CREATE INDEX idx_task_schedules_active ON task_schedules(is_active) WHERE is_active = TRUE;

-- ============================================================
-- Task instances (concrete occurrences generated from schedules)
-- ============================================================
CREATE TABLE task_instances (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    schedule_id     UUID NOT NULL REFERENCES task_schedules(id) ON DELETE CASCADE,
    template_id     UUID NOT NULL REFERENCES task_templates(id) ON DELETE CASCADE,
    assigned_to     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- When this instance is due
    due_date        DATE NOT NULL,
    window_start    TIME NOT NULL,
    window_end      TIME NOT NULL,

    status          task_instance_status NOT NULL DEFAULT 'scheduled',

    -- Make-up tracking
    is_makeup       BOOLEAN NOT NULL DEFAULT FALSE,
    original_instance_id UUID REFERENCES task_instances(id) ON DELETE SET NULL,
    makeup_deadline TIMESTAMPTZ,

    -- Fault tolerance
    missed_count_in_window INTEGER NOT NULL DEFAULT 0,

    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_task_instances_schedule ON task_instances(schedule_id);
CREATE INDEX idx_task_instances_assignee ON task_instances(assigned_to);
CREATE INDEX idx_task_instances_due ON task_instances(due_date);
CREATE INDEX idx_task_instances_status ON task_instances(status);
CREATE INDEX idx_task_instances_assignee_due ON task_instances(assigned_to, due_date);
CREATE INDEX idx_task_instances_overdue ON task_instances(status, due_date)
    WHERE status IN ('scheduled', 'in_progress');

-- ============================================================
-- Task submissions (inspector's completed work)
-- ============================================================
CREATE TABLE task_submissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id     UUID NOT NULL REFERENCES task_instances(id) ON DELETE CASCADE,
    submitted_by    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status          submission_status NOT NULL DEFAULT 'pending_review',
    notes           TEXT,
    submitted_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at     TIMESTAMPTZ,
    review_notes    TEXT
);

CREATE INDEX idx_task_submissions_instance ON task_submissions(instance_id);
CREATE INDEX idx_task_submissions_submitter ON task_submissions(submitted_by);
CREATE INDEX idx_task_submissions_status ON task_submissions(status);

-- ============================================================
-- Subtask responses (individual subtask answers)
-- ============================================================
CREATE TABLE subtask_responses (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_id   UUID NOT NULL REFERENCES task_submissions(id) ON DELETE CASCADE,
    subtask_id      UUID NOT NULL REFERENCES template_subtasks(id) ON DELETE CASCADE,
    response_value  JSONB NOT NULL,
    -- e.g., {"checked": true}, {"text": "looks good"}, {"number": 3}, {"photo_id": "uuid"}
    is_valid        BOOLEAN NOT NULL DEFAULT TRUE,
    validation_msg  TEXT,
    responded_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subtask_responses_submission ON subtask_responses(submission_id);
CREATE INDEX idx_subtask_responses_subtask ON subtask_responses(subtask_id);

-- ============================================================
-- Submission validations (per-field immediate validation results)
-- ============================================================
CREATE TABLE submission_validations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_id   UUID NOT NULL REFERENCES task_submissions(id) ON DELETE CASCADE,
    field_name      VARCHAR(255) NOT NULL,
    is_valid        BOOLEAN NOT NULL,
    message         TEXT,
    severity        VARCHAR(50) NOT NULL DEFAULT 'error',
    -- severity: 'error', 'warning', 'info'
    validated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_submission_validations_submission ON submission_validations(submission_id);

-- ============================================================
-- Task reminders (in-app reminder inbox)
-- ============================================================
CREATE TABLE task_reminders (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    instance_id     UUID REFERENCES task_instances(id) ON DELETE CASCADE,
    reminder_type   reminder_type NOT NULL,
    status          reminder_status NOT NULL DEFAULT 'unread',
    title           VARCHAR(500) NOT NULL,
    message         TEXT NOT NULL,
    due_date        DATE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    read_at         TIMESTAMPTZ,
    dismissed_at    TIMESTAMPTZ
);

CREATE INDEX idx_task_reminders_user ON task_reminders(user_id);
CREATE INDEX idx_task_reminders_status ON task_reminders(user_id, status);
CREATE INDEX idx_task_reminders_unread ON task_reminders(user_id, status)
    WHERE status = 'unread';
CREATE INDEX idx_task_reminders_instance ON task_reminders(instance_id);
