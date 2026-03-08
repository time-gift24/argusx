use std::time::Duration;

use telemetry::{TelemetryConfig, build_layer};
use tracing_subscriber::layer::SubscriberExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_flushes_high_priority_records_with_external_subscriber() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let config = TelemetryConfig {
        clickhouse_url: server.uri(),
        high_priority_batch_size: 100,
        low_priority_batch_size: 100,
        high_priority_flush_interval_ms: 60_000,
        low_priority_flush_interval_ms: 60_000,
        ..TelemetryConfig::default()
    };

    let (layer, runtime) = build_layer(config).unwrap();

    let subscriber = tracing_subscriber::registry().with(layer);
    let _guard = tracing::subscriber::set_default(subscriber);

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
}
