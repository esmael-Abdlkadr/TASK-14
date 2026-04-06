use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ── Category ────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct KbCategory {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: Option<i32>,
}

// ── Entry (head pointer) ────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct KbEntry {
    pub id: Uuid,
    pub item_name: String,
    pub category_id: Option<Uuid>,
    pub current_version: i32,
    pub region: String,
    pub is_active: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEntryRequest {
    pub item_name: String,
    pub category_id: Option<Uuid>,
    pub region: Option<String>,
    pub disposal_category: String,
    pub disposal_instructions: String,
    pub special_handling: Option<String>,
    pub contamination_notes: Option<String>,
    pub rule_source: Option<String>,
    pub effective_date: Option<NaiveDate>,
    pub aliases: Option<Vec<AliasInput>>,
    pub image_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEntryRequest {
    pub item_name: Option<String>,
    pub category_id: Option<Uuid>,
    pub region: Option<String>,
    pub disposal_category: String,
    pub disposal_instructions: String,
    pub special_handling: Option<String>,
    pub contamination_notes: Option<String>,
    pub rule_source: Option<String>,
    pub effective_date: Option<NaiveDate>,
    pub change_summary: Option<String>,
    pub aliases: Option<Vec<AliasInput>>,
    pub image_ids: Option<Vec<Uuid>>,
}

// ── Entry version (immutable) ───────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct KbEntryVersion {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub version_number: i32,
    pub item_name: String,
    pub disposal_category: String,
    pub disposal_instructions: String,
    pub special_handling: Option<String>,
    pub contamination_notes: Option<String>,
    pub region: String,
    pub rule_source: Option<String>,
    pub effective_date: NaiveDate,
    pub change_summary: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ── Aliases ─────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct KbAlias {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub alias: String,
    pub alias_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AliasInput {
    pub alias: String,
    pub alias_type: Option<String>, // alias, misspelling, abbreviation, colloquial
}

// ── Images ──────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct KbImage {
    pub id: Uuid,
    pub file_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub sha256_hash: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub uploaded_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct KbEntryVersionImage {
    pub id: Uuid,
    pub version_id: Uuid,
    pub image_id: Uuid,
    pub sort_order: i32,
    pub caption: Option<String>,
}

// ── Search ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct KbSearchQuery {
    pub q: String,
    pub region: Option<String>,
    pub category_id: Option<Uuid>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct KbSearchResult {
    pub entry_id: Uuid,
    pub item_name: String,
    pub matched_alias: Option<String>,
    pub match_type: String, // exact, prefix, fuzzy, alias_exact, alias_fuzzy
    pub score: f64,
    pub region: String,
    pub category_name: Option<String>,
    pub current_version: i32,
    pub disposal_category: String,
    pub disposal_instructions: String,
    pub special_handling: Option<String>,
    pub contamination_notes: Option<String>,
    pub rule_source: Option<String>,
    pub effective_date: NaiveDate,
    pub images: Vec<KbSearchResultImage>,
}

#[derive(Debug, Serialize)]
pub struct KbSearchResultImage {
    pub image_id: Uuid,
    pub file_name: String,
    pub url: String,
    pub caption: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct KbSearchResponse {
    pub results: Vec<KbSearchResult>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub query: String,
}

// ── Search config ───────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct KbSearchConfig {
    pub id: Uuid,
    pub name_exact_weight: f32,
    pub name_prefix_weight: f32,
    pub name_fuzzy_weight: f32,
    pub alias_exact_weight: f32,
    pub alias_fuzzy_weight: f32,
    pub category_boost: f32,
    pub region_boost: f32,
    pub recency_boost: f32,
    pub fuzzy_threshold: f32,
    pub max_results: i32,
    pub updated_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateSearchConfigRequest {
    pub name_exact_weight: Option<f32>,
    pub name_prefix_weight: Option<f32>,
    pub name_fuzzy_weight: Option<f32>,
    pub alias_exact_weight: Option<f32>,
    pub alias_fuzzy_weight: Option<f32>,
    pub category_boost: Option<f32>,
    pub region_boost: Option<f32>,
    pub recency_boost: Option<f32>,
    pub fuzzy_threshold: Option<f32>,
    pub max_results: Option<i32>,
}

// ── Version history response ────────────────────────────────

#[derive(Debug, Serialize)]
pub struct KbVersionHistoryResponse {
    pub entry_id: Uuid,
    pub item_name: String,
    pub versions: Vec<KbVersionDetail>,
}

#[derive(Debug, Serialize)]
pub struct KbVersionDetail {
    pub version: KbEntryVersion,
    pub images: Vec<KbSearchResultImage>,
    pub aliases: Vec<KbAlias>,
}
