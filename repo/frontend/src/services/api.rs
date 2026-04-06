use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

const API_BASE: &str = "/api";

/// Shared API error type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

// ── Auth types ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub role: String,
    pub username: String,
}

pub async fn login(username: &str, password: &str) -> Result<LoginResponse, String> {
    let body = serde_json::json!({"username": username, "password": password});
    let resp = Request::post(&format!("{}/auth/login", API_BASE))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        let text = resp.text().await.unwrap_or_default();
        if let Ok(err) = serde_json::from_str::<ApiError>(&text) {
            return Err(err.message);
        }
        return Err(format!("Login failed (HTTP {})", resp.status()));
    }

    let raw: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let token = raw.get("session_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let role = raw.pointer("/user/role").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let uname = raw.pointer("/user/username").and_then(|v| v.as_str()).unwrap_or(username).to_string();

    // Store token for other API modules
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.session_storage() {
            let _ = storage.set_item("session_token", &token);
        }
    }

    Ok(LoginResponse { token, role, username: uname })
}

// ── Response types matching backend models ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KbSearchResponse {
    pub results: Vec<KbSearchResult>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KbSearchResult {
    pub entry_id: Uuid,
    pub item_name: String,
    pub matched_alias: Option<String>,
    pub match_type: String,
    pub score: f64,
    pub region: String,
    pub category_name: Option<String>,
    pub current_version: i32,
    pub disposal_category: String,
    pub disposal_instructions: String,
    pub special_handling: Option<String>,
    pub contamination_notes: Option<String>,
    pub rule_source: Option<String>,
    pub effective_date: String,
    pub images: Vec<KbSearchResultImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KbSearchResultImage {
    pub image_id: Uuid,
    pub file_name: String,
    pub url: String,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KbCategory {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KbVersionHistoryResponse {
    pub entry_id: Uuid,
    pub item_name: String,
    pub versions: Vec<KbVersionDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KbVersionDetail {
    pub version: KbEntryVersion,
    pub images: Vec<KbSearchResultImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub effective_date: String,
    pub change_summary: Option<String>,
    pub created_at: String,
}

// ── API functions ───────────────────────────────────────────

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

    if !resp.ok() {
        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        if let Ok(api_err) = serde_json::from_str::<ApiError>(&error_text) {
            return Err(api_err.message);
        }
        return Err(format!("HTTP {}: {}", resp.status(), error_text));
    }

    resp.json::<T>().await.map_err(|e| e.to_string())
}

pub async fn search_kb(
    query: &str,
    region: Option<&str>,
    category_id: Option<Uuid>,
    page: i64,
    page_size: i64,
) -> Result<KbSearchResponse, String> {
    let mut url = format!(
        "{}/kb/search?q={}&page={}&page_size={}",
        API_BASE,
        urlencoded(query),
        page,
        page_size
    );

    if let Some(r) = region {
        if !r.is_empty() {
            url.push_str(&format!("&region={}", urlencoded(r)));
        }
    }

    if let Some(cid) = category_id {
        url.push_str(&format!("&category_id={}", cid));
    }

    fetch_json(&url).await
}

pub async fn get_categories() -> Result<Vec<KbCategory>, String> {
    fetch_json(&format!("{}/kb/categories", API_BASE)).await
}

pub async fn get_version_history(entry_id: Uuid) -> Result<KbVersionHistoryResponse, String> {
    fetch_json(&format!("{}/kb/entries/{}/versions", API_BASE, entry_id)).await
}

pub async fn get_search_config() -> Result<KbSearchConfig, String> {
    fetch_json(&format!("{}/kb/search-config", API_BASE)).await
}

pub async fn update_search_config(config: &KbSearchConfig) -> Result<KbSearchConfig, String> {
    let token = get_token().ok_or("Not authenticated")?;

    let resp = Request::put(&format!("{}/kb/search-config", API_BASE))
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(config).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to update config: {}", text));
    }

    resp.json().await.map_err(|e| e.to_string())
}

fn urlencoded(s: &str) -> String {
    js_sys::encode_uri_component(s).into()
}
