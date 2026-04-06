use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ── Enums ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "campaign_status", rename_all = "snake_case")]
pub enum CampaignStatus {
    Draft,
    Scheduled,
    Active,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "report_format", rename_all = "snake_case")]
pub enum ReportFormat {
    Csv,
    Pdf,
}

// ── Campaigns ───────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Campaign {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: CampaignStatus,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub target_region: Option<String>,
    pub target_audience: Option<String>,
    pub goals: Option<serde_json::Value>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCampaignRequest {
    pub name: String,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub target_region: Option<String>,
    pub target_audience: Option<String>,
    pub goals: Option<serde_json::Value>,
    pub tag_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCampaignRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<CampaignStatus>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub target_region: Option<String>,
    pub target_audience: Option<String>,
    pub goals: Option<serde_json::Value>,
    pub tag_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Serialize)]
pub struct CampaignDetail {
    pub campaign: Campaign,
    pub tags: Vec<Tag>,
}

// ── Tags ────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: Option<String>,
}

// ── KPI Metrics ─────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct KpiSnapshot {
    pub id: Uuid,
    pub metric_name: String,
    pub metric_value: f32,
    pub dimensions: Option<serde_json::Value>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DashboardKpis {
    pub sorting_conversion_rate: KpiMetric,
    pub template_reuse_rate: KpiMetric,
    pub retention_30d: KpiMetric,
    pub retention_60d: KpiMetric,
    pub retention_90d: KpiMetric,
    pub active_users: i64,
    pub total_tasks_completed: i64,
    pub total_reviews_completed: i64,
    pub total_kb_entries: i64,
    pub active_campaigns: i64,
    pub overdue_tasks: i64,
    pub pending_reviews: i64,
}

#[derive(Debug, Serialize)]
pub struct KpiMetric {
    pub current: f64,
    pub previous: f64,
    pub trend: f64,       // percentage change
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct KpiQuery {
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub region: Option<String>,
}

// ── Overview types ──────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct UserOverview {
    pub total_users: i64,
    pub by_role: Vec<RoleCount>,
    pub by_status: Vec<StatusCount>,
    pub recent_logins: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct RoleCount {
    pub role: String,
    pub count: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct StatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ItemOverview {
    pub total_kb_entries: i64,
    pub active_entries: i64,
    pub total_categories: i64,
    pub entries_by_region: Vec<RegionCount>,
    pub recent_updates: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct RegionCount {
    pub region: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct WorkOrderOverview {
    pub total_templates: i64,
    pub active_schedules: i64,
    pub total_instances: i64,
    pub by_status: Vec<InstanceStatusCount>,
    pub completion_rate: f64,
    pub avg_completion_time_hours: f64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct InstanceStatusCount {
    pub status: String,
    pub count: i64,
}

// ── Report Configs ──────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ReportConfig {
    pub id: Uuid,
    pub name: String,
    pub report_type: String,
    pub parameters: serde_json::Value,
    pub format: ReportFormat,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReportConfigRequest {
    pub name: String,
    pub report_type: String,
    pub parameters: Option<serde_json::Value>,
    pub format: Option<ReportFormat>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateReportRequest {
    pub report_type: String,
    pub format: ReportFormat,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CampaignQuery {
    pub status: Option<CampaignStatus>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}
