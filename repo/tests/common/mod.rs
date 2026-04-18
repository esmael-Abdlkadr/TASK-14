//! Shared HTTP helpers for integration tests.
//! Uses `reqwest` against `CIVICSORT_API_URL` only — no mockito/wiremock/in-process Actix stubs.

use reqwest::Client;
use std::time::Duration;
use uuid::Uuid;

pub fn base_url() -> String {
    std::env::var("CIVICSORT_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

pub fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .expect("reqwest client")
}

/// Default password meets backend validation (12+ chars, upper, lower, digit, special).
pub fn default_itest_password() -> String {
    std::env::var("CIVICSORT_ITEST_PASSWORD").unwrap_or_else(|_| "SecurePass1!xy".to_string())
}

pub fn default_itest_username() -> String {
    std::env::var("CIVICSORT_ITEST_USERNAME").unwrap_or_else(|_| "itest_admin".to_string())
}

pub fn parse_session_from_login_body(body: &str) -> (String, Uuid) {
    let v: serde_json::Value = serde_json::from_str(body).expect("login json");
    let token = v["session_token"]
        .as_str()
        .expect("session_token")
        .to_string();
    let uid_str = v["user"]["id"].as_str().expect("user.id");
    let user_id = Uuid::parse_str(uid_str).expect("user id uuid");
    (token, user_id)
}

/// Login, or bootstrap-register then login (empty DB + `CIVICSORT_BOOTSTRAP_ADMIN=1`).
pub async fn integration_admin_session() -> (String, Uuid, String) {
    let username = default_itest_username();
    let password = default_itest_password();
    let login_body = serde_json::json!({ "username": username, "password": password });
    let login_str = login_body.to_string();

    let (c, b) = post_json("/api/auth/login", &login_str, None).await;
    if c == 200 {
        let (t, u) = parse_session_from_login_body(&b);
        return (t, u, password);
    }

    let reg = serde_json::json!({
        "username": username,
        "password": password,
        "role": "OperationsAdmin"
    });
    let (c2, b2) = post_json("/api/auth/register", &reg.to_string(), None).await;
    if c2 != 201 {
        panic!(
            "integration_admin_session: login HTTP {} {}, register HTTP {} {}. \
             For a non-empty DB set CIVICSORT_ITEST_USERNAME and CIVICSORT_ITEST_PASSWORD.",
            c, b, c2, b2
        );
    }

    let (c3, b3) = post_json("/api/auth/login", &login_str, None).await;
    assert_eq!(c3, 200, "login after register: {}", b3);
    let (t, u) = parse_session_from_login_body(&b3);
    (t, u, password)
}

/// Throttle: authenticated routes share ~60 req/min per user.
pub async fn maybe_throttle_rate_limit(request_seq: &mut u32) {
    *request_seq += 1;
    if *request_seq % 40 == 0 {
        tokio::time::sleep(Duration::from_secs(62)).await;
    }
}

pub async fn post_bytes(
    path: &str,
    content_type: &str,
    body: &[u8],
    bearer: Option<&str>,
) -> (u16, String) {
    let url = format!("{}{}", base_url(), path);
    let mut req = client()
        .post(&url)
        .header("Content-Type", content_type)
        .body(body.to_vec());
    if let Some(t) = bearer {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    let resp = req.send().await.expect("POST bytes");
    let code = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    (code, text)
}

pub async fn get_json(path: &str, bearer: Option<&str>) -> (u16, String) {
    let url = format!("{}{}", base_url(), path);
    let mut req = client().get(&url);
    if let Some(t) = bearer {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    let resp = req.send().await.expect("GET request");
    let code = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    (code, body)
}

pub async fn post_json(
    path: &str,
    body_json: &str,
    bearer: Option<&str>,
) -> (u16, String) {
    let url = format!("{}{}", base_url(), path);
    let mut req = client()
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body_json.to_string());
    if let Some(t) = bearer {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    let resp = req.send().await.expect("POST request");
    let code = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    (code, body)
}

pub async fn put_json(path: &str, body_json: &str, bearer: Option<&str>) -> (u16, String) {
    let url = format!("{}{}", base_url(), path);
    let mut req = client()
        .put(&url)
        .header("Content-Type", "application/json")
        .body(body_json.to_string());
    if let Some(t) = bearer {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    let resp = req.send().await.expect("PUT request");
    let code = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    (code, body)
}

pub async fn delete_req(path: &str, bearer: Option<&str>) -> (u16, String) {
    let url = format!("{}{}", base_url(), path);
    let mut req = client().delete(&url);
    if let Some(t) = bearer {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    let resp = req.send().await.expect("DELETE request");
    let code = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    (code, body)
}

// ── Body assertion helpers ───────────────────────────────────

fn json_path<'a>(v: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut cur = v;
    for part in path.split('.') {
        cur = cur.get(part)?;
    }
    Some(cur)
}

pub fn parse_json(label: &str, body: &str) -> serde_json::Value {
    serde_json::from_str(body).unwrap_or_else(|e| {
        panic!("{}: JSON parse failed: {}\nbody: {}", label, e, &body[..body.len().min(300)])
    })
}

pub fn assert_field(label: &str, body: &str, path: &str) {
    let v = parse_json(label, body);
    let leaf = json_path(&v, path).unwrap_or_else(|| {
        panic!("{}: missing field '{}'\nbody: {}", label, path, &body[..body.len().min(400)])
    });
    assert!(
        !leaf.is_null(),
        "{}: field '{}' is null\nbody: {}",
        label,
        path,
        &body[..body.len().min(400)]
    );
}

pub fn extract_uuid(label: &str, body: &str, path: &str) -> Uuid {
    let v = parse_json(label, body);
    let s = json_path(&v, path)
        .unwrap_or_else(|| {
            panic!("{}: missing field '{}'\nbody: {}", label, path, &body[..body.len().min(400)])
        })
        .as_str()
        .unwrap_or_else(|| {
            panic!(
                "{}: field '{}' is not a string\nbody: {}",
                label,
                path,
                &body[..body.len().min(400)]
            )
        });
    Uuid::parse_str(s)
        .unwrap_or_else(|e| panic!("{}: '{}' = {:?} is not a UUID: {}", label, path, s, e))
}

pub fn find_in_list(list_body: &str, field: &str, value: &str) -> Option<serde_json::Value> {
    let v: serde_json::Value = serde_json::from_str(list_body).ok()?;
    v.as_array()?
        .iter()
        .find(|item| item.get(field).and_then(|f| f.as_str()) == Some(value))
        .cloned()
}

pub fn assert_is_array(label: &str, body: &str) {
    let v: serde_json::Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("{}: JSON parse failed: {}", label, e));
    assert!(
        v.is_array(),
        "{}: expected JSON array\nbody: {}",
        label,
        &body[..body.len().min(300)]
    );
}

pub fn assert_error_field(label: &str, body: &str) {
    let v: serde_json::Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("{}: JSON parse failed: {}", label, e));
    assert!(
        v.get("error").is_some() || v.get("message").is_some(),
        "{}: expected 'error' or 'message' field in error body\nbody: {}",
        label,
        &body[..body.len().min(400)]
    );
}

/// Application JSON error (not Actix route 404 HTML).
pub fn assert_api_surface_status(label: &str, code: u16, body: &str) {
    assert!(
        matches!(
            code,
            200 | 201 | 400 | 401 | 403 | 404 | 409 | 422 | 429
        ),
        "{}: unexpected HTTP {} — {}",
        label,
        code,
        body.chars().take(500).collect::<String>()
    );
    assert!(
        !body.contains("<!DOCTYPE") && !body.contains("<html"),
        "{}: expected JSON API body, got HTML — {}",
        label,
        body.chars().take(200).collect::<String>()
    );
}

macro_rules! api_get {
    ($path:expr, $auth:expr $(,)?) => {
        async {
            let __r: (u16, String) = $crate::common::get_json($path, $auth).await;
            __r
        }
    };
}

macro_rules! api_post {
    ($path:expr, $body:expr, $auth:expr $(,)?) => {
        async {
            let __r: (u16, String) = $crate::common::post_json($path, $body, $auth).await;
            __r
        }
    };
}

macro_rules! api_put {
    ($path:expr, $body:expr, $auth:expr $(,)?) => {
        async {
            let __r: (u16, String) = $crate::common::put_json($path, $body, $auth).await;
            __r
        }
    };
}

macro_rules! api_delete {
    ($path:expr, $auth:expr $(,)?) => {
        async {
            let __r: (u16, String) = $crate::common::delete_req($path, $auth).await;
            __r
        }
    };
}

macro_rules! api_post_bytes {
    ($path:expr, $ct:expr, $body:expr, $auth:expr $(,)?) => {
        async {
            let __r: (u16, String) = $crate::common::post_bytes($path, $ct, $body, $auth).await;
            __r
        }
    };
}
