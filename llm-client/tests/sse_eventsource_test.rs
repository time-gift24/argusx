use futures::StreamExt;
use llm_client::sse::{Event, EventSource};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn event_source_from_response_emits_open_and_messages() {
    let mock_server = MockServer::start().await;

    let body = "data: {\"hello\":\"world\"}\n\n";
    Mock::given(method("GET"))
        .and(path("/events"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&mock_server)
        .await;

    let response = reqwest::Client::new()
        .get(format!("{}/events", mock_server.uri()))
        .send()
        .await
        .expect("response");

    let mut es = EventSource::from_response(response).expect("event source");

    let open = es.next().await.expect("open event").expect("ok");
    assert!(matches!(open, Event::Open));

    let msg = es.next().await.expect("message event").expect("ok");
    match msg {
        Event::Message(msg) => {
            assert_eq!(msg.event, "message");
            assert_eq!(msg.data, "{\"hello\":\"world\"}");
        }
        Event::Open => panic!("expected message"),
    }
}

#[tokio::test]
async fn event_source_supports_trailing_event_without_blank_line() {
    let mock_server = MockServer::start().await;

    let body = "data: tail event\n";
    Mock::given(method("GET"))
        .and(path("/events"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&mock_server)
        .await;

    let response = reqwest::Client::new()
        .get(format!("{}/events", mock_server.uri()))
        .send()
        .await
        .expect("response");

    let mut es = EventSource::from_response(response).expect("event source");
    let _ = es.next().await;

    let msg = es.next().await.expect("message event").expect("ok");
    match msg {
        Event::Message(msg) => assert_eq!(msg.data, "tail event"),
        Event::Open => panic!("expected message"),
    }
}
