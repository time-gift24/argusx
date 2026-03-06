#[test]
fn provider_crate_exposes_mapper() {
    let _ = provider::VERSION;
    let _ = provider::Mapper::new(provider::Dialect::Openai);
}
