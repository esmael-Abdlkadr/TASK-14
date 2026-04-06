use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

const API_BASE: &str = "/api/messaging";

fn get_token() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.session_storage().ok()??;
    storage.get_item("session_token").ok()?
}

async fn fetch_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let mut req = Request::get(url);
    if let Some(token) = get_token() {
        req = req.header("Authorization", &format!("Bearer {}", token));
    }
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

// ── Types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Notification {
    pub id: Uuid,
    pub channel: String,
    pub subject: Option<String>,
    pub body: String,
    pub status: String,
    pub event_type: Option<String>,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub created_at: String,
    pub read_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationInbox {
    pub unread_count: i64,
    pub notifications: Vec<Notification>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalPayload {
    pub id: Uuid,
    pub channel: String,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub status: String,
    pub retry_count: i32,
    pub max_retries: i32,
    pub last_error: Option<String>,
    pub export_path: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayloadQueueResponse {
    pub payloads: Vec<ExternalPayload>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub queued_count: i64,
    pub failed_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeliveryLogEntry {
    pub id: Uuid,
    pub action: String,
    pub status_after: String,
    pub details: Option<String>,
    pub performed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationTemplate {
    pub id: Uuid,
    pub name: String,
    pub channel: String,
    pub body_template: String,
    pub subject_template: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TriggerRule {
    pub id: Uuid,
    pub name: String,
    pub event: String,
    pub template_id: Uuid,
    pub channel: String,
    pub is_active: bool,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportBatchResult {
    pub channel: String,
    pub count: usize,
    pub export_dir: String,
    pub files: Vec<String>,
}

// ── API calls ───────────────────────────────────────────────

pub async fn get_notifications(status: Option<&str>, page: i64) -> Result<NotificationInbox, String> {
    let mut url = format!("{}/notifications?page={}&page_size=20", API_BASE, page);
    if let Some(s) = status { url.push_str(&format!("&status={}", s)); }
    fetch_json(&url).await
}

pub async fn mark_notification_read(id: Uuid) -> Result<(), String> {
    post_json::<_, serde_json::Value>(&format!("{}/notifications/{}/read", API_BASE, id), &serde_json::json!({})).await?;
    Ok(())
}

pub async fn dismiss_notification(id: Uuid) -> Result<(), String> {
    post_json::<_, serde_json::Value>(&format!("{}/notifications/{}/dismiss", API_BASE, id), &serde_json::json!({})).await?;
    Ok(())
}

pub async fn mark_all_read() -> Result<serde_json::Value, String> {
    post_json(&format!("{}/notifications/read-all", API_BASE), &serde_json::json!({})).await
}

pub async fn get_templates() -> Result<Vec<NotificationTemplate>, String> {
    fetch_json(&format!("{}/templates", API_BASE)).await
}

pub async fn get_triggers() -> Result<Vec<TriggerRule>, String> {
    fetch_json(&format!("{}/triggers", API_BASE)).await
}

pub async fn get_payload_queue(status: Option<&str>, page: i64) -> Result<PayloadQueueResponse, String> {
    let mut url = format!("{}/payloads?page={}&page_size=20", API_BASE, page);
    if let Some(s) = status { url.push_str(&format!("&status={}", s)); }
    fetch_json(&url).await
}

pub async fn export_payloads(channel: &str) -> Result<ExportBatchResult, String> {
    post_json(&format!("{}/payloads/export", API_BASE), &serde_json::json!({"channel": channel})).await
}

pub async fn mark_delivered(ids: Vec<Uuid>) -> Result<serde_json::Value, String> {
    post_json(&format!("{}/payloads/mark-delivered", API_BASE), &serde_json::json!({"payload_ids": ids})).await
}

pub async fn mark_failed(id: Uuid, error: &str) -> Result<serde_json::Value, String> {
    post_json(&format!("{}/payloads/mark-failed", API_BASE), &serde_json::json!({"payload_id": id, "error": error})).await
}

pub async fn get_delivery_log(id: Uuid) -> Result<Vec<DeliveryLogEntry>, String> {
    fetch_json(&format!("{}/payloads/{}/log", API_BASE, id)).await
}
