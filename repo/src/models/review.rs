use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ── Enums ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "review_target_type", rename_all = "snake_case")]
pub enum ReviewTargetType {
    InspectionSubmission,
    DisputedClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "assignment_method", rename_all = "snake_case")]
pub enum AssignmentMethod {
    Automatic,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "review_assignment_status", rename_all = "snake_case")]
pub enum ReviewAssignmentStatus {
    Pending,
    InProgress,
    Completed,
    Recused,
    Reassigned,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "review_status", rename_all = "snake_case")]
pub enum ReviewStatus {
    Draft,
    Submitted,
    Finalized,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "consistency_severity", rename_all = "snake_case")]
pub enum ConsistencySeverity {
    Warning,
    Error,
}

// ── Scorecards ──────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Scorecard {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_type: ReviewTargetType,
    pub is_active: bool,
    pub passing_score: Option<f32>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateScorecardRequest {
    pub name: String,
    pub description: Option<String>,
    pub target_type: ReviewTargetType,
    pub passing_score: Option<f32>,
    pub dimensions: Option<Vec<CreateDimensionInput>>,
    pub consistency_rules: Option<Vec<CreateConsistencyRuleInput>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateScorecardRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub passing_score: Option<f32>,
}

// ── Dimensions ──────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ScorecardDimension {
    pub id: Uuid,
    pub scorecard_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub weight: f32,
    pub sort_order: i32,
    pub rating_levels: serde_json::Value,
    pub comment_required: bool,
    pub comment_required_below: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDimensionInput {
    pub name: String,
    pub description: Option<String>,
    pub weight: Option<f32>,
    pub sort_order: Option<i32>,
    pub rating_levels: Option<serde_json::Value>,
    pub comment_required: Option<bool>,
    pub comment_required_below: Option<i32>,
}

// ── Consistency Rules ───────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ConsistencyRule {
    pub id: Uuid,
    pub scorecard_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub severity: ConsistencySeverity,
    pub dimension_a_id: Uuid,
    pub range_a_min: i32,
    pub range_a_max: i32,
    pub dimension_b_id: Uuid,
    pub range_b_min: i32,
    pub range_b_max: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateConsistencyRuleInput {
    pub name: String,
    pub description: Option<String>,
    pub severity: Option<ConsistencySeverity>,
    pub dimension_a_id: Uuid,
    pub range_a_min: i32,
    pub range_a_max: i32,
    pub dimension_b_id: Uuid,
    pub range_b_min: i32,
    pub range_b_max: i32,
}

// ── Assignments ─────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ReviewAssignment {
    pub id: Uuid,
    pub reviewer_id: Uuid,
    pub target_type: ReviewTargetType,
    pub target_id: Uuid,
    pub scorecard_id: Uuid,
    pub method: AssignmentMethod,
    pub status: ReviewAssignmentStatus,
    pub is_blind: bool,
    pub recused_at: Option<DateTime<Utc>>,
    pub recusal_reason: Option<String>,
    pub reassigned_from: Option<Uuid>,
    pub assigned_by: Option<Uuid>,
    pub assigned_at: DateTime<Utc>,
    pub due_date: Option<NaiveDate>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAssignmentRequest {
    pub reviewer_id: Option<Uuid>,       // None = auto-assign
    pub target_type: ReviewTargetType,
    pub target_id: Uuid,
    pub scorecard_id: Uuid,
    pub is_blind: Option<bool>,
    pub due_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct RecusalRequest {
    pub reason: String,
}

/// Assignment enriched for display — blind reviews anonymize submitter info
#[derive(Debug, Serialize)]
pub struct AssignmentDetail {
    pub assignment: ReviewAssignment,
    pub scorecard: Scorecard,
    pub dimensions: Vec<ScorecardDimension>,
    pub consistency_rules: Vec<ConsistencyRule>,
    pub target_summary: TargetSummary,
    pub existing_review: Option<Review>,
}

/// Summary of the review target — anonymized if blind
#[derive(Debug, Serialize)]
pub struct TargetSummary {
    pub target_type: ReviewTargetType,
    pub target_id: Uuid,
    pub title: String,
    pub submitted_at: Option<String>,
    pub submitter_name: Option<String>,  // None if blind
    pub details: serde_json::Value,
}

// ── Reviews ─────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Review {
    pub id: Uuid,
    pub assignment_id: Uuid,
    pub reviewer_id: Uuid,
    pub scorecard_id: Uuid,
    pub target_type: ReviewTargetType,
    pub target_id: Uuid,
    pub status: ReviewStatus,
    pub overall_score: Option<f32>,
    pub overall_comment: Option<String>,
    pub recommendation: Option<String>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub finalized_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ReviewScore {
    pub id: Uuid,
    pub review_id: Uuid,
    pub dimension_id: Uuid,
    pub rating: i32,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitReviewRequest {
    pub scores: Vec<ScoreInput>,
    pub overall_comment: Option<String>,
    pub recommendation: String,          // approve, reject, revise
    pub acknowledge_warnings: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ScoreInput {
    pub dimension_id: Uuid,
    pub rating: i32,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReviewDetail {
    pub review: Review,
    pub scores: Vec<ReviewScore>,
    pub consistency_results: Vec<ConsistencyCheckResult>,
}

// ── Consistency Check Results ───────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ConsistencyCheckResult {
    pub id: Uuid,
    pub review_id: Uuid,
    pub rule_id: Uuid,
    pub severity: ConsistencySeverity,
    pub message: String,
    pub acknowledged: bool,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ConsistencyCheckOutput {
    pub has_errors: bool,
    pub has_warnings: bool,
    pub results: Vec<ConsistencyCheckItem>,
}

#[derive(Debug, Serialize)]
pub struct ConsistencyCheckItem {
    pub rule_name: String,
    pub severity: ConsistencySeverity,
    pub message: String,
    pub dimension_a: String,
    pub dimension_b: String,
}

// ── Conflict of Interest ────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ConflictOfInterest {
    pub id: Uuid,
    pub reviewer_id: Uuid,
    pub conflict_type: String,
    pub target_user_id: Option<Uuid>,
    pub department: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub declared_at: DateTime<Utc>,
    pub declared_by: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct DeclareCoiRequest {
    pub conflict_type: String,
    pub target_user_id: Option<Uuid>,
    pub department: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ReviewerDepartment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub department: String,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
}

// ── Review Queue ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ReviewQueueQuery {
    pub status: Option<ReviewAssignmentStatus>,
    pub target_type: Option<ReviewTargetType>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ReviewQueueResponse {
    pub assignments: Vec<AssignmentDetail>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Serialize)]
pub struct ScorecardWithDimensions {
    pub scorecard: Scorecard,
    pub dimensions: Vec<ScorecardDimension>,
    pub consistency_rules: Vec<ConsistencyRule>,
}
