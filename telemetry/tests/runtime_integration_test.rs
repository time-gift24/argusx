use std::time::Duration;

use telemetry::{TelemetryConfig, init};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_flushes_high_priority_records_on_shutdown() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut config = TelemetryConfig::default();
    config.clickhouse_url = server.uri();
    config.high_priority_batch_size = 100;
    config.low_priority_batch_size = 100;
    config.high_priority_flush_interval_ms = 60_000;
    config.low_priority_flush_interval_ms = 60_000;

    let runtime = init(config).unwrap();

    tracing::info!(
        event_name = "turn_finished",
        event_priority = "high",
        session_id = "session-1",
        turn_id = "turn-1",
        sequence_no = 1u64
    );

    runtime.shutdown(Duration::from_secs(2)).unwrap();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(String::from_utf8_lossy(&requests[0].body).contains("turn_finished"));
}
