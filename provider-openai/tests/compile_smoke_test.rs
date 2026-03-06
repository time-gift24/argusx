#[test]
fn crates_compile() {
    let _ = core::ResponseEvent::Done(None);
    let _ = provider_openai::VERSION;
}
