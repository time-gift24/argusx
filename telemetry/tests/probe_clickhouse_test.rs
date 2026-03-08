use telemetry::{probe_clickhouse, TelemetryConfig};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn probe_succeeds_against_healthy_clickhouse() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .and(query_param("query", "SELECT 1"))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .mount(&server)
        .await;

    let config = TelemetryConfig {
        clickhouse_url: server.uri(),
        ..TelemetryConfig::default()
    };

    probe_clickhouse(&config).await.unwrap();
}

#[tokio::test]
async fn probe_fails_on_non_success_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let config = TelemetryConfig {
        clickhouse_url: server.uri(),
        ..TelemetryConfig::default()
    };

    let err = probe_clickhouse(&config).await.unwrap_err();
    assert!(err.to_string().contains("503"));
}
