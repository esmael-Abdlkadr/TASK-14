//! Health & basic HTTP wiring.

#[tokio::test]
async fn get_health_returns_200_with_expected_json() {
    let (status, body) = api_get!("/api/health", None).await;
    assert_eq!(status, 200, "body: {}", body);
    let v: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["status"], "healthy");
    assert_eq!(v["service"], "civicsort");
}

#[tokio::test]
async fn unknown_api_path_returns_404() {
    let (status, _body) = api_get!("/api/nonexistent", None).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn auth_session_without_token_returns_401() {
    let (status, _body) = api_get!("/api/auth/session", None).await;
    assert_eq!(status, 401);
}
