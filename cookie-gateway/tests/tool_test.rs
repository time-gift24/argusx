use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use cookie_gateway::error::CookieGatewayError;
use cookie_gateway::tool::{CookieCommandClient, CookieFetchSource, CookieFetchTool};
use cookie_gateway::{CookieData, CookieStore};

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

#[derive(Clone)]
struct MockCommandClient {
    calls: Arc<AtomicUsize>,
    cookies: Vec<CookieData>,
    delay: Duration,
    fail: bool,
}

impl MockCommandClient {
    fn success(calls: Arc<AtomicUsize>, cookies: Vec<CookieData>, delay: Duration) -> Self {
        Self {
            calls,
            cookies,
            delay,
            fail: false,
        }
    }

    fn fail(calls: Arc<AtomicUsize>) -> Self {
        Self {
            calls,
            cookies: vec![],
            delay: Duration::from_millis(0),
            fail: true,
        }
    }
}

#[async_trait]
impl CookieCommandClient for MockCommandClient {
    async fn request_cookies(
        &self,
        _domain: &str,
        _timeout: Duration,
    ) -> Result<Vec<CookieData>, CookieGatewayError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }

        if self.fail {
            return Err(CookieGatewayError::ExtensionClientUnavailable);
        }

        Ok(self.cookies.clone())
    }
}

#[tokio::test]
async fn fetch_returns_cache_when_fresh() {
    let store = Arc::new(CookieStore::new());
    store
        .store_cookies(
            "api.company.com",
            vec![sample_cookie("api.company.com", "cached")],
        )
        .await;

    let calls = Arc::new(AtomicUsize::new(0));
    let command_client = Arc::new(MockCommandClient::success(
        calls.clone(),
        vec![sample_cookie("api.company.com", "from-client")],
        Duration::from_millis(0),
    ));

    let tool = CookieFetchTool::new(store, command_client, Duration::from_secs(2));
    let output = tool
        .fetch("api.company.com", Duration::from_secs(60))
        .await
        .unwrap();

    assert!(matches!(output.source, CookieFetchSource::Cache));
    assert_eq!(output.count, 1);
    assert_eq!(output.cookies[0].value, "cached");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn fetch_refreshes_when_stale() {
    let store = Arc::new(CookieStore::new());
    store
        .store_cookies_with_fetched_at(
            "api.company.com",
            vec![sample_cookie("api.company.com", "stale")],
            SystemTime::now() - Duration::from_secs(3_600),
        )
        .await;

    let calls = Arc::new(AtomicUsize::new(0));
    let command_client = Arc::new(MockCommandClient::success(
        calls.clone(),
        vec![sample_cookie("api.company.com", "fresh")],
        Duration::from_millis(0),
    ));

    let tool = CookieFetchTool::new(store.clone(), command_client, Duration::from_secs(2));
    let output = tool
        .fetch("api.company.com", Duration::from_secs(1))
        .await
        .unwrap();

    assert!(matches!(output.source, CookieFetchSource::Refresh));
    assert_eq!(output.cookies[0].value, "fresh");
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let cached = store.get_cookies("api.company.com").await.unwrap();
    assert_eq!(cached[0].value, "fresh");
}

#[tokio::test]
async fn fetch_returns_error_when_missing_and_client_unavailable() {
    let store = Arc::new(CookieStore::new());

    let calls = Arc::new(AtomicUsize::new(0));
    let command_client = Arc::new(MockCommandClient::fail(calls.clone()));

    let tool = CookieFetchTool::new(store, command_client, Duration::from_secs(2));
    let err = tool
        .fetch("api.company.com", Duration::from_secs(10))
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        CookieGatewayError::ExtensionClientUnavailable
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn fetch_deduplicates_refresh_requests_for_same_domain() {
    let store = Arc::new(CookieStore::new());

    let calls = Arc::new(AtomicUsize::new(0));
    let command_client = Arc::new(MockCommandClient::success(
        calls.clone(),
        vec![sample_cookie("api.company.com", "fresh")],
        Duration::from_millis(120),
    ));

    let tool = CookieFetchTool::new(store, command_client, Duration::from_secs(2));
    let tool_a = tool.clone();
    let tool_b = tool.clone();

    let (result_a, result_b) = tokio::join!(
        tokio::spawn(async move {
            tool_a
                .fetch("api.company.com", Duration::from_secs(1))
                .await
        }),
        tokio::spawn(async move {
            tool_b
                .fetch("api.company.com", Duration::from_secs(1))
                .await
        })
    );

    let first = result_a.unwrap().unwrap();
    let second = result_b.unwrap().unwrap();

    assert!(matches!(
        first.source,
        CookieFetchSource::Refresh | CookieFetchSource::Cache
    ));
    assert!(matches!(
        second.source,
        CookieFetchSource::Refresh | CookieFetchSource::Cache
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
