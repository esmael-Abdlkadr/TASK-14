use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "dispute_status", rename_all = "snake_case")]
pub enum DisputeStatus {
    Open,
    UnderReview,
    Resolved,
    Dismissed,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ClassificationDispute {
    pub id: Uuid,
    pub kb_entry_id: Uuid,
    pub disputed_by: Uuid,
    pub reason: String,
    pub proposed_category: Option<String>,
    pub proposed_instructions: Option<String>,
    pub status: DisputeStatus,
    pub resolution_notes: Option<String>,
    pub resolved_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDisputeRequest {
    pub kb_entry_id: Uuid,
    pub reason: String,
    pub proposed_category: Option<String>,
    pub proposed_instructions: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveDisputeRequest {
    pub status: DisputeStatus,
    pub resolution_notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DisputeDetail {
    #[serde(flatten)]
    pub dispute: ClassificationDispute,
    pub kb_entry_name: String,
}
