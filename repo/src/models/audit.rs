use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::UserRole;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub event_id: Uuid,
    pub user_id: Option<Uuid>,
    pub username: String,
    pub role: Option<UserRole>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing)]
    pub encrypted_details: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub session_id: Option<Uuid>,
    pub prev_hash: Option<String>,
    pub entry_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub user_id: Option<Uuid>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AuditExportQuery {
    pub format: ExportFormat,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub action: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Csv,
    Pdf,
}

#[derive(Debug, Serialize)]
pub struct AuditLogPage {
    pub entries: Vec<AuditLogEntry>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
