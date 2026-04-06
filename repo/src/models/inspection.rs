use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ── Enums ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "task_cycle", rename_all = "snake_case")]
pub enum TaskCycle {
    Daily,
    Weekly,
    Biweekly,
    Monthly,
    Quarterly,
    OneTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "task_instance_status", rename_all = "snake_case")]
pub enum TaskInstanceStatus {
    Scheduled,
    InProgress,
    Submitted,
    Completed,
    Overdue,
    Missed,
    Makeup,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "submission_status", rename_all = "snake_case")]
pub enum SubmissionStatus {
    PendingReview,
    Approved,
    Rejected,
    NeedsRevision,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "reminder_type", rename_all = "snake_case")]
pub enum ReminderType {
    Upcoming,
    DueSoon,
    Overdue,
    MakeupDeadline,
    MissedWarning,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "reminder_status", rename_all = "snake_case")]
pub enum ReminderStatus {
    Unread,
    Read,
    Dismissed,
}

// ── Task Template ───────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TaskTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub group_name: Option<String>,
    pub cycle: TaskCycle,
    pub time_window_start: NaiveTime,
    pub time_window_end: NaiveTime,
    pub allowed_misses: i32,
    pub miss_window_days: i32,
    pub makeup_allowed: bool,
    pub makeup_deadline_hours: i32,
    pub is_active: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub group_name: Option<String>,
    pub cycle: TaskCycle,
    pub time_window_start: Option<String>, // "HH:MM"
    pub time_window_end: Option<String>,
    pub allowed_misses: Option<i32>,
    pub miss_window_days: Option<i32>,
    pub makeup_allowed: Option<bool>,
    pub makeup_deadline_hours: Option<i32>,
    pub subtasks: Option<Vec<CreateSubtaskInput>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub group_name: Option<String>,
    pub cycle: Option<TaskCycle>,
    pub time_window_start: Option<String>,
    pub time_window_end: Option<String>,
    pub allowed_misses: Option<i32>,
    pub miss_window_days: Option<i32>,
    pub makeup_allowed: Option<bool>,
    pub makeup_deadline_hours: Option<i32>,
}

// ── Subtasks ────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TemplateSubtask {
    pub id: Uuid,
    pub template_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub is_required: bool,
    pub expected_type: String,
    pub options: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSubtaskInput {
    pub title: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
    pub is_required: Option<bool>,
    pub expected_type: Option<String>,
    pub options: Option<serde_json::Value>,
}

// ── Schedules ───────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TaskSchedule {
    pub id: Uuid,
    pub template_id: Uuid,
    pub assigned_to: Uuid,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub is_active: bool,
    pub notes: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateScheduleRequest {
    pub template_id: Uuid,
    pub assigned_to: Uuid,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub notes: Option<String>,
}

// ── Task Instances ──────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TaskInstance {
    pub id: Uuid,
    pub schedule_id: Uuid,
    pub template_id: Uuid,
    pub assigned_to: Uuid,
    pub due_date: NaiveDate,
    pub window_start: NaiveTime,
    pub window_end: NaiveTime,
    pub status: TaskInstanceStatus,
    pub is_makeup: bool,
    pub original_instance_id: Option<Uuid>,
    pub makeup_deadline: Option<DateTime<Utc>>,
    pub missed_count_in_window: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Enriched instance with template details for display
#[derive(Debug, Serialize)]
pub struct TaskInstanceDetail {
    pub instance: TaskInstance,
    pub template_name: String,
    pub template_group: Option<String>,
    pub subtasks: Vec<TemplateSubtask>,
    pub submission: Option<TaskSubmission>,
}

// ── Submissions ─────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TaskSubmission {
    pub id: Uuid,
    pub instance_id: Uuid,
    pub submitted_by: Uuid,
    pub status: SubmissionStatus,
    pub notes: Option<String>,
    #[serde(skip_serializing)]
    pub encrypted_notes: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSubmissionRequest {
    pub instance_id: Uuid,
    pub notes: Option<String>,
    pub responses: Vec<SubtaskResponseInput>,
}

#[derive(Debug, Deserialize)]
pub struct SubtaskResponseInput {
    pub subtask_id: Uuid,
    pub response_value: serde_json::Value,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SubtaskResponse {
    pub id: Uuid,
    pub submission_id: Uuid,
    pub subtask_id: Uuid,
    pub response_value: serde_json::Value,
    pub is_valid: bool,
    pub validation_msg: Option<String>,
    pub responded_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewSubmissionRequest {
    pub status: SubmissionStatus,
    pub review_notes: Option<String>,
}

// ── Validation ──────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SubmissionValidation {
    pub id: Uuid,
    pub submission_id: Uuid,
    pub field_name: String,
    pub is_valid: bool,
    pub message: Option<String>,
    pub severity: String,
    pub validated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationItem>,
    pub warnings: Vec<ValidationItem>,
}

#[derive(Debug, Serialize)]
pub struct ValidationItem {
    pub field: String,
    pub message: String,
}

// ── Reminders ───────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TaskReminder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub instance_id: Option<Uuid>,
    pub reminder_type: ReminderType,
    pub status: ReminderStatus,
    pub title: String,
    pub message: String,
    pub due_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub dismissed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ReminderQuery {
    pub status: Option<ReminderStatus>,
    pub reminder_type: Option<ReminderType>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ReminderInbox {
    pub unread_count: i64,
    pub reminders: Vec<TaskReminder>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

// ── Dashboard views ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TaskListQuery {
    pub status: Option<TaskInstanceStatus>,
    pub due_date: Option<NaiveDate>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskInstanceDetail>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Serialize)]
pub struct TemplateWithSubtasks {
    pub template: TaskTemplate,
    pub subtasks: Vec<TemplateSubtask>,
}

#[derive(Debug, Serialize)]
pub struct SubmissionDetail {
    pub submission: TaskSubmission,
    pub responses: Vec<SubtaskResponse>,
    pub validations: Vec<SubmissionValidation>,
}
