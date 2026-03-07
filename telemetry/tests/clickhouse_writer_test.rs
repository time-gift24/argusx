use telemetry::{ClickHouseWriter, EventPriority, TelemetryConfig, TelemetryRecord};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn writer_posts_json_each_row_to_clickhouse() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let config = TelemetryConfig {
        clickhouse_url: server.uri(),
        ..TelemetryConfig::default()
    };
    let writer = ClickHouseWriter::new(config).unwrap();

    writer
        .write_batch(vec![
            TelemetryRecord::builder("turn_finished", EventPriority::High).build(),
        ])
        .await
        .unwrap();
}

#[tokio::test]
async fn writer_does_not_retry_validation_errors() {
    let writer = ClickHouseWriter::new(TelemetryConfig::default()).unwrap();
    let invalid = TelemetryRecord::builder("llm_response_completed", EventPriority::High).build();

    let err = writer.write_batch(vec![invalid]).await.unwrap_err();
    assert!(err.to_string().contains("billing_dedupe_key"));
}
