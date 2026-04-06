use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

const API_BASE: &str = "/api/bulk";

fn get_token() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.session_storage().ok()??;
    storage.get_item("session_token").ok()?
}

async fn fetch_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let mut req = Request::get(url);
    if let Some(token) = get_token() { req = req.header("Authorization", &format!("Bearer {}", token)); }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() { return Err(resp.text().await.unwrap_or_default()); }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

async fn post_json<B: Serialize, T: DeserializeOwned>(url: &str, body: &B) -> Result<T, String> {
    let token = get_token().ok_or("Not authenticated")?;
    let resp = Request::post(url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send().await.map_err(|e| e.to_string())?;
    if !resp.ok() { return Err(resp.text().await.unwrap_or_default()); }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

async fn put_json<B: Serialize, T: DeserializeOwned>(url: &str, body: &B) -> Result<T, String> {
    let token = get_token().ok_or("Not authenticated")?;
    let resp = Request::put(url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send().await.map_err(|e| e.to_string())?;
    if !resp.ok() { return Err(resp.text().await.unwrap_or_default()); }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

// ── Types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportJob { pub id: Uuid, pub name: String, pub entity_type: String, pub total_rows: i32, pub imported_rows: i32, pub duplicate_rows: i32, pub error_rows: i32, pub status: String, pub created_at: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportRow { pub id: Uuid, pub row_number: i32, pub raw_data: serde_json::Value, pub status: String, pub duplicate_of: Option<Uuid>, pub error_message: Option<String> }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportResult { pub job: ImportJob, pub rows: Vec<ImportRow>, pub duplicates_found: usize, pub errors_found: usize }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataChange { pub id: Uuid, pub entity_type: String, pub entity_id: Uuid, pub operation: String, pub field_name: Option<String>, pub old_value: Option<serde_json::Value>, pub new_value: Option<serde_json::Value>, pub changed_by: Uuid, pub changed_at: String, pub reverted_at: Option<String> }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeHistoryResponse { pub changes: Vec<DataChange>, pub total: i64, pub page: i64, pub page_size: i64 }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DuplicateFlag { pub id: Uuid, pub entity_type: String, pub source_id: Uuid, pub target_id: Uuid, pub match_type: String, pub confidence: f32, pub status: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeRequest { pub id: Uuid, pub entity_type: String, pub source_id: Uuid, pub target_id: Uuid, pub status: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeConflict { pub id: Uuid, pub field_name: String, pub source_value: Option<serde_json::Value>, pub target_value: Option<serde_json::Value>, pub resolution: Option<String> }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeRequestDetail { pub request: MergeRequest, pub conflicts: Vec<MergeConflict> }

// ── API calls ───────────────────────────────────────────────

pub async fn start_import(name: &str, entity_type: &str, rows: Vec<serde_json::Value>) -> Result<ImportResult, String> {
    post_json(&format!("{}/import", API_BASE), &serde_json::json!({"name": name, "entity_type": entity_type, "rows": rows})).await
}
pub async fn execute_import(id: Uuid) -> Result<ImportJob, String> {
    post_json(&format!("{}/import/{}/execute", API_BASE, id), &serde_json::json!({})).await
}
pub async fn list_imports() -> Result<Vec<ImportJob>, String> { fetch_json(&format!("{}/import", API_BASE)).await }
pub async fn get_import(id: Uuid) -> Result<serde_json::Value, String> { fetch_json(&format!("{}/import/{}", API_BASE, id)).await }
pub async fn get_changes(entity_type: Option<&str>, page: i64) -> Result<ChangeHistoryResponse, String> {
    let mut url = format!("{}/changes?page={}&page_size=50", API_BASE, page);
    if let Some(et) = entity_type { url.push_str(&format!("&entity_type={}", et)); }
    fetch_json(&url).await
}
pub async fn revert_change(id: Uuid) -> Result<DataChange, String> {
    post_json(&format!("{}/changes/{}/revert", API_BASE, id), &serde_json::json!({})).await
}
pub async fn get_duplicates(status: Option<&str>) -> Result<Vec<DuplicateFlag>, String> {
    let mut url = format!("{}/duplicates?", API_BASE);
    if let Some(s) = status { url.push_str(&format!("status={}&", s)); }
    fetch_json(&url).await
}
pub async fn resolve_duplicate(id: Uuid, status: &str) -> Result<DuplicateFlag, String> {
    put_json(&format!("{}/duplicates/{}/resolve", API_BASE, id), &serde_json::json!({"status": status})).await
}
pub async fn get_merge_requests() -> Result<Vec<MergeRequest>, String> { fetch_json(&format!("{}/merges", API_BASE)).await }
pub async fn get_merge_detail(id: Uuid) -> Result<MergeRequestDetail, String> { fetch_json(&format!("{}/merges/{}", API_BASE, id)).await }
pub async fn resolve_conflict(merge_id: Uuid, conflict_id: Uuid, resolution: &str, custom: Option<serde_json::Value>) -> Result<MergeConflict, String> {
    put_json(&format!("{}/merges/{}/conflicts/{}", API_BASE, merge_id, conflict_id), &serde_json::json!({"resolution": resolution, "custom_value": custom})).await
}
pub async fn review_merge(id: Uuid, status: &str, notes: Option<&str>) -> Result<MergeRequest, String> {
    put_json(&format!("{}/merges/{}/review", API_BASE, id), &serde_json::json!({"status": status, "review_notes": notes})).await
}
