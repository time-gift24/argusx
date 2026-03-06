#[test]
fn crates_compile() {
    let _ = argus_core::ResponseEvent::Done(None);
    let _ = provider_openai::VERSION;
}
