use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

const API_BASE: &str = "/api/admin";

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
pub struct KpiMetric {
    pub current: f64,
    pub previous: f64,
    pub trend: f64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserOverview {
    pub total_users: i64,
    pub by_role: Vec<CountItem>,
    pub by_status: Vec<CountItem>,
    pub recent_logins: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemOverview {
    pub total_kb_entries: i64,
    pub active_entries: i64,
    pub total_categories: i64,
    pub entries_by_region: Vec<CountItem>,
    pub recent_updates: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkOrderOverview {
    pub total_templates: i64,
    pub active_schedules: i64,
    pub total_instances: i64,
    pub by_status: Vec<CountItem>,
    pub completion_rate: f64,
    pub avg_completion_time_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CountItem {
    #[serde(alias = "role", alias = "status", alias = "region")]
    pub label: Option<String>,
    pub count: i64,
    // Accept multiple field name variants
    pub role: Option<String>,
    pub status: Option<String>,
    pub region: Option<String>,
}

impl CountItem {
    pub fn display_label(&self) -> String {
        self.label.clone()
            .or_else(|| self.role.clone())
            .or_else(|| self.status.clone())
            .or_else(|| self.region.clone())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Campaign {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub start_date: String,
    pub end_date: String,
    pub target_region: Option<String>,
    pub target_audience: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CampaignDetail {
    pub campaign: Campaign,
    pub tags: Vec<Tag>,
}

// ── API calls ───────────────────────────────────────────────

pub async fn get_dashboard(from: Option<&str>, to: Option<&str>) -> Result<DashboardKpis, String> {
    let mut url = format!("{}/dashboard?", API_BASE);
    if let Some(f) = from { url.push_str(&format!("from_date={}&", f)); }
    if let Some(t) = to { url.push_str(&format!("to_date={}&", t)); }
    fetch_json(&url).await
}

pub async fn get_user_overview() -> Result<UserOverview, String> {
    fetch_json(&format!("{}/overview/users", API_BASE)).await
}

pub async fn get_item_overview() -> Result<ItemOverview, String> {
    fetch_json(&format!("{}/overview/items", API_BASE)).await
}

pub async fn get_workorder_overview() -> Result<WorkOrderOverview, String> {
    fetch_json(&format!("{}/overview/workorders", API_BASE)).await
}

pub async fn get_campaigns(status: Option<&str>, page: i64) -> Result<Vec<Campaign>, String> {
    let mut url = format!("{}/campaigns?page={}", API_BASE, page);
    if let Some(s) = status { url.push_str(&format!("&status={}", s)); }
    fetch_json(&url).await
}

pub async fn get_campaign(id: Uuid) -> Result<CampaignDetail, String> {
    fetch_json(&format!("{}/campaigns/{}", API_BASE, id)).await
}

pub async fn create_campaign(body: serde_json::Value) -> Result<CampaignDetail, String> {
    post_json(&format!("{}/campaigns", API_BASE), &body).await
}

pub async fn get_tags() -> Result<Vec<Tag>, String> {
    fetch_json(&format!("{}/tags", API_BASE)).await
}

pub async fn create_tag(name: &str, color: Option<&str>) -> Result<Tag, String> {
    post_json(&format!("{}/tags", API_BASE), &serde_json::json!({"name": name, "color": color})).await
}

pub async fn generate_report(report_type: &str, format: &str, from: Option<&str>, to: Option<&str>) -> Result<(), String> {
    let token = get_token().ok_or("Not authenticated")?;
    let body = serde_json::json!({
        "report_type": report_type,
        "format": format,
        "from_date": from,
        "to_date": to,
    });
    let resp = Request::post(&format!("{}/reports/generate", API_BASE))
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send().await.map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(resp.text().await.unwrap_or_default());
    }

    // Trigger browser download from the response blob
    let blob = resp.binary().await.map_err(|e| e.to_string())?;
    trigger_download(&blob, &format!("{}_{}.{}", report_type, chrono::Utc::now().format("%Y%m%d"), format));
    Ok(())
}

fn trigger_download(data: &[u8], filename: &str) {
    use js_sys::{Array, Uint8Array};
    use web_sys::{Blob, BlobPropertyBag, Url};

    let uint8arr = Uint8Array::new_with_length(data.len() as u32);
    uint8arr.copy_from(data);
    let array = Array::new();
    array.push(&uint8arr.buffer());

    let blob = Blob::new_with_buffer_source_sequence_and_options(
        &array, BlobPropertyBag::new().type_("application/octet-stream"),
    ).unwrap();

    let url = Url::create_object_url_with_blob(&blob).unwrap();
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let a = document.create_element("a").unwrap();
    a.set_attribute("href", &url).unwrap();
    a.set_attribute("download", filename).unwrap();
    a.set_attribute("style", "display:none").unwrap();
    document.body().unwrap().append_child(&a).unwrap();
    let html_a: web_sys::HtmlElement = a.dyn_into().unwrap();
    html_a.click();
    html_a.remove();
    Url::revoke_object_url(&url).unwrap();
}
