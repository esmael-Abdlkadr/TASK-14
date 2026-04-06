use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ── Enums ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "import_job_status", rename_all = "snake_case")]
pub enum ImportJobStatus {
    Pending,
    Validating,
    Validated,
    Importing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "import_row_status", rename_all = "snake_case")]
pub enum ImportRowStatus {
    Pending,
    Valid,
    Duplicate,
    Conflict,
    Imported,
    Skipped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "change_operation", rename_all = "snake_case")]
pub enum ChangeOperation {
    Create,
    Update,
    Delete,
    Merge,
    Import,
    Revert,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "merge_request_status", rename_all = "snake_case")]
pub enum MergeRequestStatus {
    Pending,
    Approved,
    Rejected,
    Applied,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "duplicate_status", rename_all = "snake_case")]
pub enum DuplicateStatus {
    Detected,
    Confirmed,
    Dismissed,
    Merged,
}

// ── Import Jobs ─────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ImportJob {
    pub id: Uuid,
    pub name: String,
    pub entity_type: String,
    pub file_name: Option<String>,
    pub total_rows: i32,
    pub processed_rows: i32,
    pub imported_rows: i32,
    pub duplicate_rows: i32,
    pub error_rows: i32,
    pub status: ImportJobStatus,
    pub error_message: Option<String>,
    pub imported_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ImportRow {
    pub id: Uuid,
    pub job_id: Uuid,
    pub row_number: i32,
    pub raw_data: serde_json::Value,
    pub parsed_data: Option<serde_json::Value>,
    pub status: ImportRowStatus,
    pub entity_id: Option<Uuid>,
    pub duplicate_of: Option<Uuid>,
    pub error_message: Option<String>,
    pub validation_errors: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct StartImportRequest {
    pub name: String,
    pub entity_type: String,
    pub rows: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ImportValidationResult {
    pub job: ImportJob,
    pub rows: Vec<ImportRow>,
    pub duplicates_found: usize,
    pub errors_found: usize,
}

// ── Data Changes ────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DataChange {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub operation: ChangeOperation,
    pub field_name: Option<String>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub change_set_id: Option<Uuid>,
    pub import_job_id: Option<Uuid>,
    pub merge_request_id: Option<Uuid>,
    pub changed_by: Uuid,
    pub changed_at: DateTime<Utc>,
    pub reverted_at: Option<DateTime<Utc>>,
    pub reverted_by: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeHistoryQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub operation: Option<ChangeOperation>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChangeHistoryResponse {
    pub changes: Vec<DataChange>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

// ── Content Fingerprints ────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ContentFingerprint {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub fingerprint_type: String,
    pub fingerprint: String,
    pub source_text: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Duplicate Flags ─────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DuplicateFlag {
    pub id: Uuid,
    pub entity_type: String,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub match_type: String,
    pub confidence: f32,
    pub status: DuplicateStatus,
    pub details: Option<serde_json::Value>,
    pub detected_at: DateTime<Utc>,
    pub resolved_by: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct DuplicateQuery {
    pub entity_type: Option<String>,
    pub status: Option<DuplicateStatus>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveDuplicateRequest {
    pub status: DuplicateStatus,
}

// ── Merge Requests ──────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct MergeRequest {
    pub id: Uuid,
    pub entity_type: String,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub duplicate_flag_id: Option<Uuid>,
    pub status: MergeRequestStatus,
    pub resolution: Option<serde_json::Value>,
    pub provenance: Option<serde_json::Value>,
    pub requested_by: Uuid,
    pub reviewed_by: Option<Uuid>,
    pub review_notes: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub applied_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct MergeConflict {
    pub id: Uuid,
    pub merge_request_id: Uuid,
    pub field_name: String,
    pub source_value: Option<serde_json::Value>,
    pub target_value: Option<serde_json::Value>,
    pub resolution: Option<String>,
    pub custom_value: Option<serde_json::Value>,
    pub resolved_by: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMergeRequest {
    pub entity_type: String,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub duplicate_flag_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveConflictRequest {
    pub resolution: String,
    pub custom_value: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewMergeRequest {
    pub status: MergeRequestStatus,
    pub review_notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MergeRequestDetail {
    pub request: MergeRequest,
    pub conflicts: Vec<MergeConflict>,
}

// ── Export ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ExportDataRequest {
    pub entity_type: String,
    pub format: String, // csv, json
    pub include_history: Option<bool>,
}
