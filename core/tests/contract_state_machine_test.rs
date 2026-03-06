use core::{ResponseContract, ResponseEvent, Usage};

#[test]
fn terminal_event_is_exclusive() {
    let mut c = ResponseContract::new();
    assert!(c.accept(ResponseEvent::Done(Some(Usage::zero()))).is_ok());
    assert!(c.accept(ResponseEvent::Error("late".into())).is_err());
}
