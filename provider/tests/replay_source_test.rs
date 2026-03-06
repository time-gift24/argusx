use std::path::PathBuf;

use futures::StreamExt;
use provider::{ReplayReader, ReplayTiming};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[tokio::test]
async fn replay_fast_mode_reads_frames_in_order() {
    let path = fixture_path("2026-03-07-sample-replay.sse");
    let mut stream = ReplayReader::open(path, ReplayTiming::Fast).await.unwrap();

    assert_eq!(
        stream.next().await.unwrap().unwrap(),
        "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n"
    );
    assert_eq!(stream.next().await.unwrap().unwrap(), "data: [DONE]\n\n");
    assert!(stream.next().await.is_none());
}

#[tokio::test(start_paused = true)]
async fn replay_recorded_mode_uses_sidecar_offsets() {
    let path = fixture_path("2026-03-07-sample-replay.sse");
    let mut stream = ReplayReader::open(path, ReplayTiming::Recorded).await.unwrap();

    let first = stream.next().await.unwrap().unwrap();
    assert!(first.starts_with("data: "));

    tokio::time::advance(std::time::Duration::from_millis(15)).await;
    let second = stream.next().await.unwrap().unwrap();
    assert_eq!(second, "data: [DONE]\n\n");
}
