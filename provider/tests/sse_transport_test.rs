use futures::StreamExt;
use provider::transport::sse::{Event, EventSource};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn event_source_reads_message_and_done_boundary() {
    let server = MockServer::start().await;
    let body =
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"g\",\"choices\":[]}\n\n";

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body, "text/event-stream"),
        )
        .mount(&server)
        .await;

    let response = reqwest::Client::new()
        .post(format!("{}/chat/completions", server.uri()))
        .send()
        .await
        .unwrap();

    let mut es = EventSource::from_response(response).unwrap();
    assert!(matches!(es.next().await, Some(Ok(Event::Open))));
    assert!(matches!(es.next().await, Some(Ok(Event::Message(_)))));
}
