use tempfile::tempdir;

use provider::SseRecorder;

#[tokio::test]
async fn recorder_writes_canonical_sse_and_sidecar() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("capture.sse");
    let mut recorder = SseRecorder::create(file.clone(), true).await.unwrap();

    recorder.write_frame("data: {\"id\":\"1\"}\n\n").await.unwrap();
    recorder.write_frame("data: [DONE]\n\n").await.unwrap();
    recorder.finish().await.unwrap();

    let body = tokio::fs::read_to_string(&file).await.unwrap();
    assert_eq!(body, "data: {\"id\":\"1\"}\n\ndata: [DONE]\n\n");

    let sidecar = tokio::fs::read_to_string(file.with_extension("sse.meta.json"))
        .await
        .unwrap();
    assert!(sidecar.contains("\"frame_index\":0"));
    assert!(sidecar.contains("\"frame_index\":1"));
}
