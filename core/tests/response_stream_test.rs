use core::{FinishReason, ResponseEvent, ResponseStream};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::task;

#[test]
fn response_stream_yields_events_in_order() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (tx, rx) = mpsc::channel(4);
            let producer = task::spawn(async move {
                tx.send(ResponseEvent::ContentDelta("hi".into()))
                    .await
                    .unwrap();
                tx.send(ResponseEvent::Done {
                    reason: FinishReason::Stop,
                    usage: None,
                })
                .await
                .unwrap();
            });

            let mut stream = ResponseStream::from_parts(rx, producer.abort_handle());
            assert!(matches!(
                stream.next().await,
                Some(ResponseEvent::ContentDelta(_))
            ));
            assert!(matches!(
                stream.next().await,
                Some(ResponseEvent::Done {
                    reason: FinishReason::Stop,
                    usage: None,
                })
            ));
            assert!(stream.next().await.is_none());
        });
}

#[test]
fn dropping_response_stream_aborts_producer() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (_tx, rx) = mpsc::channel::<ResponseEvent>(1);
            let producer = task::spawn(async {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            });

            let stream = ResponseStream::from_parts(rx, producer.abort_handle());
            drop(stream);
            let err = producer.await.unwrap_err();
            assert!(err.is_cancelled());
        });
}
