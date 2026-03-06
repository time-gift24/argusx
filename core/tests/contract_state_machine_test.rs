use core::{ResponseContract, ResponseEvent, Usage};

#[test]
fn terminal_event_is_exclusive() {
    let mut c = ResponseContract::new();
    assert!(c.accept(&ResponseEvent::Done(Some(Usage::zero()))).is_ok());
    assert!(c.accept(&ResponseEvent::Error("late".into())).is_err());
}

#[test]
fn delta_after_terminal_is_rejected() {
    let mut c = ResponseContract::new();
    assert!(c.accept(&ResponseEvent::Done(None)).is_ok());
    assert!(c
        .accept(&ResponseEvent::ContentDelta("late".into()))
        .is_err());
}

#[test]
fn done_after_error_is_rejected() {
    let mut c = ResponseContract::new();
    assert!(c.accept(&ResponseEvent::Error("boom".into())).is_ok());
    assert!(c.accept(&ResponseEvent::Done(None)).is_err());
}
