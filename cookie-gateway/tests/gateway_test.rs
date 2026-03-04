use axum::body::Body;
use axum::http::StatusCode;
use axum::http::{Method, Request};
use cookie_gateway::gateway::{app, GatewayState};
use cookie_gateway::CookieData;
use cookie_gateway::CookieStore;
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::ServiceExt;

fn sample_cookie(domain: &str, value: &str) -> CookieData {
    CookieData {
        name: "session".to_string(),
        value: value.to_string(),
        domain: domain.to_string(),
        path: "/".to_string(),
        secure: true,
        http_only: true,
        expiration_date: None,
    }
}

#[tokio::test]
async fn test_health_endpoint() {
    let state = GatewayState::new();
    let app = app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
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
    store.set_opt_in(true).await;
    let state = GatewayState::with_store(Arc::new(store));
    let app = app(state);

    let payload = serde_json::json!({
        "domain": "api.company.com",
        "cookies": vec![sample_cookie("api.company.com", "abc123")]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/cookies")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_upload_cookies_rejects_non_whitelisted_domain() {
    let store = CookieStore::new();
    let state = GatewayState::with_store(Arc::new(store));
    let app = app(state);

    let payload = serde_json::json!({
        "domain": "malicious.com",
        "cookies": vec![sample_cookie("malicious.com", "evil")]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/cookies")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_cookies() {
    let store = CookieStore::new();
    store.set_opt_in(true).await;
    let state = GatewayState::with_store(Arc::new(store));
    let app = app(state);

    let upload_payload = serde_json::json!({
        "domain": "api.company.com",
        "cookies": vec![sample_cookie("api.company.com", "abc123")]
    });

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/cookies")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&upload_payload).unwrap()))
                .unwrap(),
        )
        .await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/cookies?domain=api.company.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["cookies"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_fetch_cookies_returns_cache_when_fresh() {
    let store = CookieStore::new();
    store.set_opt_in(true).await;
    store
        .store_cookies(
            "api.company.com",
            vec![sample_cookie("api.company.com", "cached")],
        )
        .await;

    let state = GatewayState::with_store(Arc::new(store));
    let app = app(state);

    let payload = serde_json::json!({
        "domain": "api.company.com",
        "refresh_after_ms": 60_000
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/cookies/fetch")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["source"], "cache");
    assert_eq!(body["count"], 1);
}

#[tokio::test]
async fn test_fetch_cookies_returns_503_without_connected_extension() {
    let store = CookieStore::new();
    store.set_opt_in(true).await;
    let state = GatewayState::with_store(Arc::new(store));
    let app = app(state);

    let payload = serde_json::json!({
        "domain": "api.company.com",
        "refresh_after_ms": 1
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/cookies/fetch")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_fetch_tool_accepts_non_whitelisted_domain() {
    let store = CookieStore::new();
    store.set_opt_in(true).await;
    let state = GatewayState::with_store(Arc::new(store));
    let app = app(state);

    let payload = serde_json::json!({
        "domain": "github.com",
        "refresh_after_ms": 1
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/cookies/fetch")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
