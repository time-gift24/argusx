use std::path::PathBuf;

use provider::{Dialect, ProviderConfig, ProviderDevOptions, ProviderStreamMode, ReplayTiming};

#[test]
fn provider_config_defaults_to_live_mode() {
    let cfg = ProviderConfig::new(Dialect::Openai, "http://localhost", "test-key");
    assert!(cfg.dev.is_none());
}

#[test]
fn provider_config_can_enable_replay_mode() {
    let cfg = ProviderConfig::new(Dialect::Openai, "http://localhost", "test-key")
        .with_dev_options(ProviderDevOptions::replay(
            PathBuf::from("/tmp/sample.sse"),
            ReplayTiming::Fast,
        ));

    assert!(matches!(
        cfg.dev.as_ref().unwrap().stream_mode,
        ProviderStreamMode::Replay {
            timing: ReplayTiming::Fast,
            ..
        }
    ));
}
