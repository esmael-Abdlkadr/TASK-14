use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

const API_BASE: &str = "/api/inspection";

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
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", resp.status(), text));
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
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

// ── Types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub group_name: Option<String>,
    pub cycle: String,
    pub time_window_start: String,
    pub time_window_end: String,
    pub allowed_misses: i32,
    pub miss_window_days: i32,
    pub makeup_allowed: bool,
    pub makeup_deadline_hours: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateSubtask {
    pub id: Uuid,
    pub template_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub is_required: bool,
    pub expected_type: String,
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateWithSubtasks {
    pub template: TaskTemplate,
    pub subtasks: Vec<TemplateSubtask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskInstance {
    pub id: Uuid,
    pub schedule_id: Uuid,
    pub template_id: Uuid,
    pub assigned_to: Uuid,
    pub due_date: String,
    pub window_start: String,
    pub window_end: String,
    pub status: String,
    pub is_makeup: bool,
    pub makeup_deadline: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskSubmission {
    pub id: Uuid,
    pub instance_id: Uuid,
    pub submitted_by: Uuid,
    pub status: String,
    pub notes: Option<String>,
    pub submitted_at: String,
    pub reviewed_by: Option<Uuid>,
    pub review_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskInstanceDetail {
    pub instance: TaskInstance,
    pub template_name: String,
    pub template_group: Option<String>,
    pub subtasks: Vec<TemplateSubtask>,
    pub submission: Option<TaskSubmission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskInstanceDetail>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskReminder {
    pub id: Uuid,
    pub reminder_type: String,
    pub status: String,
    pub title: String,
    pub message: String,
    pub due_date: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReminderInbox {
    pub unread_count: i64,
    pub reminders: Vec<TaskReminder>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationItem {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationItem>,
    pub warnings: Vec<ValidationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubmissionResponse {
    pub valid: bool,
    pub validation: ValidationResult,
    pub submission: Option<serde_json::Value>,
}

// ── API calls ───────────────────────────────────────────────

pub async fn get_templates() -> Result<Vec<TaskTemplate>, String> {
    fetch_json(&format!("{}/templates", API_BASE)).await
}

pub async fn get_template(id: Uuid) -> Result<TemplateWithSubtasks, String> {
    fetch_json(&format!("{}/templates/{}", API_BASE, id)).await
}

pub async fn get_tasks(
    status: Option<&str>,
    from_date: Option<&str>,
    to_date: Option<&str>,
    page: i64,
) -> Result<TaskListResponse, String> {
    let mut url = format!("{}/tasks?page={}&page_size=20", API_BASE, page);
    if let Some(s) = status {
        url.push_str(&format!("&status={}", s));
    }
    if let Some(f) = from_date {
        url.push_str(&format!("&from_date={}", f));
    }
    if let Some(t) = to_date {
        url.push_str(&format!("&to_date={}", t));
    }
    fetch_json(&url).await
}

pub async fn get_task(id: Uuid) -> Result<TaskInstanceDetail, String> {
    fetch_json(&format!("{}/tasks/{}", API_BASE, id)).await
}

pub async fn start_task(id: Uuid) -> Result<TaskInstance, String> {
    post_json(&format!("{}/tasks/{}/start", API_BASE, id), &serde_json::json!({})).await
}

pub async fn submit_task(
    instance_id: Uuid,
    notes: Option<String>,
    responses: Vec<serde_json::Value>,
) -> Result<SubmissionResponse, String> {
    let body = serde_json::json!({
        "instance_id": instance_id,
        "notes": notes,
        "responses": responses,
    });
    post_json(&format!("{}/submissions", API_BASE), &body).await
}

pub async fn get_reminders(status: Option<&str>, page: i64) -> Result<ReminderInbox, String> {
    let mut url = format!("{}/reminders?page={}&page_size=20", API_BASE, page);
    if let Some(s) = status {
        url.push_str(&format!("&status={}", s));
    }
    fetch_json(&url).await
}

pub async fn mark_reminder_read(id: Uuid) -> Result<(), String> {
    post_json::<_, serde_json::Value>(
        &format!("{}/reminders/{}/read", API_BASE, id),
        &serde_json::json!({}),
    )
    .await?;
    Ok(())
}

pub async fn dismiss_reminder(id: Uuid) -> Result<(), String> {
    post_json::<_, serde_json::Value>(
        &format!("{}/reminders/{}/dismiss", API_BASE, id),
        &serde_json::json!({}),
    )
    .await?;
    Ok(())
}

pub async fn mark_all_read() -> Result<serde_json::Value, String> {
    post_json(&format!("{}/reminders/read-all", API_BASE), &serde_json::json!({})).await
}
