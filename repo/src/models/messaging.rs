use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ── Enums ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "notification_channel", rename_all = "snake_case")]
pub enum NotificationChannel {
    InApp,
    Sms,
    Email,
    Push,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "notification_status", rename_all = "snake_case")]
pub enum NotificationStatus {
    Pending,
    Delivered,
    Read,
    Failed,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "payload_status", rename_all = "snake_case")]
pub enum PayloadStatus {
    Queued,
    Exported,
    Delivered,
    Failed,
    Retrying,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "trigger_event", rename_all = "snake_case")]
pub enum TriggerEvent {
    InspectionScheduled,
    InspectionStarted,
    InspectionSubmitted,
    InspectionOverdue,
    InspectionMissed,
    TaskRescheduled,
    ReviewAssigned,
    ReviewCompleted,
    ReviewRecused,
    AppealSubmitted,
    AppealOutcome,
    ReminderUpcoming,
    ReminderDueSoon,
    CampaignStarted,
    CampaignEnding,
    UserRegistered,
    AccountLocked,
    Custom,
}

// ── Notification Templates ──────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct NotificationTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub channel: NotificationChannel,
    pub subject_template: Option<String>,
    pub body_template: String,
    pub sms_template: Option<String>,
    pub html_template: Option<String>,
    pub is_active: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotifTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub channel: NotificationChannel,
    pub subject_template: Option<String>,
    pub body_template: String,
    pub sms_template: Option<String>,
    pub html_template: Option<String>,
    pub variables: Option<Vec<TemplateVariableInput>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotifTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub channel: Option<NotificationChannel>,
    pub subject_template: Option<String>,
    pub body_template: Option<String>,
    pub sms_template: Option<String>,
    pub html_template: Option<String>,
}

// ── Template Variables ──────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TemplateVariable {
    pub id: Uuid,
    pub template_id: Uuid,
    pub var_name: String,
    pub var_type: String,
    pub description: Option<String>,
    pub default_value: Option<String>,
    pub is_required: bool,
}

#[derive(Debug, Deserialize)]
pub struct TemplateVariableInput {
    pub var_name: String,
    pub var_type: Option<String>,
    pub description: Option<String>,
    pub default_value: Option<String>,
    pub is_required: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct TemplateWithVariables {
    pub template: NotificationTemplate,
    pub variables: Vec<TemplateVariable>,
}

// ── Trigger Rules ───────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TriggerRule {
    pub id: Uuid,
    pub name: String,
    pub event: TriggerEvent,
    pub template_id: Uuid,
    pub channel: NotificationChannel,
    pub conditions: Option<serde_json::Value>,
    pub target_role: Option<String>,
    pub is_active: bool,
    pub priority: i32,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTriggerRuleRequest {
    pub name: String,
    pub event: TriggerEvent,
    pub template_id: Uuid,
    pub channel: Option<NotificationChannel>,
    pub conditions: Option<serde_json::Value>,
    pub target_role: Option<String>,
    pub priority: Option<i32>,
}

// ── Notifications ───────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub template_id: Option<Uuid>,
    pub trigger_rule_id: Option<Uuid>,
    pub channel: NotificationChannel,
    pub subject: Option<String>,
    pub body: String,
    pub rendered_data: Option<serde_json::Value>,
    pub status: NotificationStatus,
    pub event_type: Option<TriggerEvent>,
    pub event_payload: Option<serde_json::Value>,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub read_at: Option<DateTime<Utc>>,
    pub dismissed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct NotificationQuery {
    pub status: Option<NotificationStatus>,
    pub channel: Option<NotificationChannel>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct NotificationInbox {
    pub unread_count: i64,
    pub notifications: Vec<Notification>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

// ── External Payloads ───────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ExternalPayload {
    pub id: Uuid,
    pub notification_id: Option<Uuid>,
    pub channel: NotificationChannel,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub metadata: Option<serde_json::Value>,
    pub export_path: Option<String>,
    pub exported_at: Option<DateTime<Utc>>,
    pub status: PayloadStatus,
    pub retry_count: i32,
    pub max_retries: i32,
    pub last_error: Option<String>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PayloadQueueQuery {
    pub status: Option<PayloadStatus>,
    pub channel: Option<NotificationChannel>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PayloadQueueResponse {
    pub payloads: Vec<ExternalPayload>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub queued_count: i64,
    pub failed_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct MarkDeliveredRequest {
    pub payload_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct MarkFailedRequest {
    pub payload_id: Uuid,
    pub error: String,
}

// ── Delivery Log ────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DeliveryLogEntry {
    pub id: Uuid,
    pub payload_id: Uuid,
    pub action: String,
    pub status_before: Option<PayloadStatus>,
    pub status_after: PayloadStatus,
    pub details: Option<String>,
    pub performed_by: Option<Uuid>,
    pub performed_at: DateTime<Utc>,
}

// ── Event dispatch ──────────────────────────────────────────

/// Input for firing a notification event
#[derive(Debug, Deserialize, Clone)]
pub struct FireEventRequest {
    pub event: TriggerEvent,
    pub payload: serde_json::Value,
    pub recipient_user_id: Option<Uuid>,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
}

/// Result of processing an event through trigger rules
#[derive(Debug, Serialize)]
pub struct FireEventResult {
    pub event: TriggerEvent,
    pub rules_matched: usize,
    pub notifications_created: usize,
    pub payloads_queued: usize,
}
