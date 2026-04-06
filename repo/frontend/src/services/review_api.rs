use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

const API_BASE: &str = "/api/reviews";

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
        return Err(resp.text().await.unwrap_or_default());
    }
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
    if !resp.ok() {
        return Err(resp.text().await.unwrap_or_default());
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

// ── Types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scorecard {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_type: String,
    pub passing_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScorecardDimension {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub weight: f32,
    pub sort_order: i32,
    pub rating_levels: serde_json::Value,
    pub comment_required: bool,
    pub comment_required_below: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsistencyRule {
    pub id: Uuid,
    pub name: String,
    pub severity: String,
    pub dimension_a_id: Uuid,
    pub dimension_b_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewAssignment {
    pub id: Uuid,
    pub reviewer_id: Uuid,
    pub target_type: String,
    pub target_id: Uuid,
    pub scorecard_id: Uuid,
    pub method: String,
    pub status: String,
    pub is_blind: bool,
    pub due_date: Option<String>,
    pub assigned_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TargetSummary {
    pub target_type: String,
    pub target_id: Uuid,
    pub title: String,
    pub submitted_at: Option<String>,
    pub submitter_name: Option<String>,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Review {
    pub id: Uuid,
    pub status: String,
    pub overall_score: Option<f32>,
    pub overall_comment: Option<String>,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssignmentDetail {
    pub assignment: ReviewAssignment,
    pub scorecard: Scorecard,
    pub dimensions: Vec<ScorecardDimension>,
    pub consistency_rules: Vec<ConsistencyRule>,
    pub target_summary: TargetSummary,
    pub existing_review: Option<Review>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewQueueResponse {
    pub assignments: Vec<AssignmentDetail>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsistencyCheckItem {
    pub rule_name: String,
    pub severity: String,
    pub message: String,
    pub dimension_a: String,
    pub dimension_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsistencyCheckOutput {
    pub has_errors: bool,
    pub has_warnings: bool,
    pub results: Vec<ConsistencyCheckItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubmitReviewResponse {
    pub valid: bool,
    pub consistency: ConsistencyCheckOutput,
    pub review: Option<serde_json::Value>,
    pub message: Option<String>,
}

// ── API calls ───────────────────────────────────────────────

pub async fn get_review_queue(status: Option<&str>, page: i64) -> Result<ReviewQueueResponse, String> {
    let mut url = format!("{}/queue?page={}&page_size=20", API_BASE, page);
    if let Some(s) = status {
        url.push_str(&format!("&status={}", s));
    }
    fetch_json(&url).await
}

pub async fn get_assignment_detail(id: Uuid) -> Result<AssignmentDetail, String> {
    fetch_json(&format!("{}/assignments/{}", API_BASE, id)).await
}

pub async fn submit_review(
    assignment_id: Uuid,
    scores: Vec<serde_json::Value>,
    overall_comment: Option<String>,
    recommendation: String,
    acknowledge_warnings: bool,
) -> Result<SubmitReviewResponse, String> {
    let body = serde_json::json!({
        "scores": scores,
        "overall_comment": overall_comment,
        "recommendation": recommendation,
        "acknowledge_warnings": acknowledge_warnings,
    });
    post_json(&format!("{}/assignments/{}/submit", API_BASE, assignment_id), &body).await
}

pub async fn recuse_assignment(id: Uuid, reason: &str) -> Result<serde_json::Value, String> {
    post_json(
        &format!("{}/assignments/{}/recuse", API_BASE, id),
        &serde_json::json!({"reason": reason}),
    ).await
}

pub async fn get_coi_list() -> Result<Vec<serde_json::Value>, String> {
    fetch_json(&format!("{}/coi", API_BASE)).await
}
