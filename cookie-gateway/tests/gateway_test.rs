use axum::http::StatusCode;
use axum::body::Body;
use axum::http::{Request, Method};
use cookie_gateway::gateway::{app, GatewayState};
use cookie_gateway::CookieData;
use tower::ServiceExt;
use http_body_util::BodyExt;
use std::sync::Arc;
use cookie_gateway::CookieStore;

#[tokio::test]
async fn test_health_endpoint() {
    let state = GatewayState::new();
    let app = app(state);

    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn test_upload_cookies() {
    let store = CookieStore::new();
    store.set_opt_in(true).await;  // Enable opt-in for testing
    let state = GatewayState { store: Arc::new(store) };
    let app = app(state);

    let payload = serde_json::json!({
        "domain": "api.company.com",
        "cookies": vec![CookieData {
            name: "session".to_string(),
            value: "abc123".to_string(),
            domain: "api.company.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: true,
            expiration_date: None,
        }]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/cookies")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_upload_cookies_rejects_non_whitelisted_domain() {
    let store = CookieStore::new();
    let state = GatewayState { store: Arc::new(store) };
    let app = app(state);

    let payload = serde_json::json!({
        "domain": "malicious.com",
        "cookies": vec![CookieData {
            name: "session".to_string(),
            value: "evil".to_string(),
            domain: "malicious.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            expiration_date: None,
        }]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/cookies")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_cookies() {
    let store = CookieStore::new();
    store.set_opt_in(true).await;  // Enable opt-in for testing
    let state = GatewayState { store: Arc::new(store) };
    let app = app(state);

    // First upload some cookies
    let upload_payload = serde_json::json!({
        "domain": "api.company.com",
        "cookies": vec![CookieData {
            name: "session".to_string(),
            value: "abc123".to_string(),
            domain: "api.company.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: true,
            expiration_date: None,
        }]
    });

    let _ = app.clone()  // Handle the Result
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/cookies")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&upload_payload).unwrap()))
                .unwrap()
        )
        .await;

    // Now retrieve them
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/cookies?domain=api.company.com")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["cookies"].as_array().unwrap().len(), 1);
}
