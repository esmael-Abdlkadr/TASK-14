-- CivicSort: Messaging & Notification Center
-- Migration 006: Template engine, trigger rules, notifications, external payloads

-- ============================================================
-- Enums
-- ============================================================

CREATE TYPE notification_channel AS ENUM (
    'in_app',
    'sms',
    'email',
    'push'
);

CREATE TYPE notification_status AS ENUM (
    'pending',
    'delivered',
    'read',
    'failed',
    'dismissed'
);

CREATE TYPE payload_status AS ENUM (
    'queued',
    'exported',
    'delivered',
    'failed',
    'retrying'
);

CREATE TYPE trigger_event AS ENUM (
    'inspection_scheduled',
    'inspection_started',
    'inspection_submitted',
    'inspection_overdue',
    'inspection_missed',
    'task_rescheduled',
    'review_assigned',
    'review_completed',
    'review_recused',
    'appeal_submitted',
    'appeal_outcome',
    'reminder_upcoming',
    'reminder_due_soon',
    'campaign_started',
    'campaign_ending',
    'user_registered',
    'account_locked',
    'custom'
);

-- ============================================================
-- Notification templates (with variable placeholders)
-- ============================================================
CREATE TABLE notification_templates (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(500) NOT NULL UNIQUE,
    description     TEXT,
    channel         notification_channel NOT NULL DEFAULT 'in_app',

    -- Template body with {{variable}} placeholders
    subject_template TEXT,
    body_template   TEXT NOT NULL,

    -- For SMS: short body; for email: HTML body
    sms_template    TEXT,
    html_template   TEXT,

    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notif_templates_channel ON notification_templates(channel);
CREATE INDEX idx_notif_templates_active ON notification_templates(is_active)
    WHERE is_active = TRUE;

-- ============================================================
-- Template variables (documents expected variables per template)
-- ============================================================
CREATE TABLE template_variables (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_id     UUID NOT NULL REFERENCES notification_templates(id) ON DELETE CASCADE,
    var_name        VARCHAR(255) NOT NULL,
    var_type        VARCHAR(50) NOT NULL DEFAULT 'string',
    -- var_type: 'string', 'date', 'number', 'url', 'user_name'
    description     TEXT,
    default_value   TEXT,
    is_required     BOOLEAN NOT NULL DEFAULT TRUE,
    UNIQUE(template_id, var_name)
);

CREATE INDEX idx_template_vars_template ON template_variables(template_id);

-- ============================================================
-- Trigger rules (map events to templates)
-- ============================================================
CREATE TABLE trigger_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(500) NOT NULL,
    event           trigger_event NOT NULL,
    template_id     UUID NOT NULL REFERENCES notification_templates(id) ON DELETE CASCADE,
    channel         notification_channel NOT NULL DEFAULT 'in_app',

    -- Optional conditions (JSONB filter on event payload)
    conditions      JSONB,
    -- e.g., {"region": "north", "task_cycle": "daily"}

    -- Target audience
    target_role     VARCHAR(50),
    -- NULL = event's natural recipient; or 'operations_admin', etc.

    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    priority        INTEGER NOT NULL DEFAULT 0,
    created_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_trigger_rules_event ON trigger_rules(event);
CREATE INDEX idx_trigger_rules_active ON trigger_rules(is_active) WHERE is_active = TRUE;

-- ============================================================
-- Notifications (generated in-app messages)
-- ============================================================
CREATE TABLE notifications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    template_id     UUID REFERENCES notification_templates(id) ON DELETE SET NULL,
    trigger_rule_id UUID REFERENCES trigger_rules(id) ON DELETE SET NULL,
    channel         notification_channel NOT NULL DEFAULT 'in_app',

    subject         TEXT,
    body            TEXT NOT NULL,
    rendered_data   JSONB,

    status          notification_status NOT NULL DEFAULT 'pending',
    event_type      trigger_event,
    event_payload   JSONB,
    reference_type  VARCHAR(100),
    reference_id    UUID,

    delivered_at    TIMESTAMPTZ,
    read_at         TIMESTAMPTZ,
    dismissed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_user ON notifications(user_id);
CREATE INDEX idx_notifications_status ON notifications(user_id, status);
CREATE INDEX idx_notifications_unread ON notifications(user_id, status)
    WHERE status IN ('pending', 'delivered');
CREATE INDEX idx_notifications_event ON notifications(event_type);
CREATE INDEX idx_notifications_ref ON notifications(reference_type, reference_id);

-- ============================================================
-- External payloads (queued SMS/email/push for manual transfer)
-- ============================================================
CREATE TABLE external_payloads (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    notification_id UUID REFERENCES notifications(id) ON DELETE SET NULL,
    channel         notification_channel NOT NULL,

    -- Payload content
    recipient       TEXT NOT NULL,
    subject         TEXT,
    body            TEXT NOT NULL,
    metadata        JSONB,

    -- File export tracking
    export_path     TEXT,
    exported_at     TIMESTAMPTZ,

    status          payload_status NOT NULL DEFAULT 'queued',
    retry_count     INTEGER NOT NULL DEFAULT 0,
    max_retries     INTEGER NOT NULL DEFAULT 3,
    last_error      TEXT,
    next_retry_at   TIMESTAMPTZ,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ext_payloads_status ON external_payloads(status);
CREATE INDEX idx_ext_payloads_channel ON external_payloads(channel, status);
CREATE INDEX idx_ext_payloads_retry ON external_payloads(next_retry_at)
    WHERE status = 'retrying';

-- ============================================================
-- Delivery tracking log (immutable history)
-- ============================================================
CREATE TABLE delivery_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payload_id      UUID NOT NULL REFERENCES external_payloads(id) ON DELETE CASCADE,
    action          VARCHAR(100) NOT NULL,
    -- action: 'queued', 'exported', 'transfer_attempted', 'delivered', 'failed', 'retry_scheduled'
    status_before   payload_status,
    status_after    payload_status NOT NULL,
    details         TEXT,
    performed_by    UUID REFERENCES users(id) ON DELETE SET NULL,
    performed_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_delivery_log_payload ON delivery_log(payload_id);
