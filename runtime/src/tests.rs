use crate::{AppConfig, ArgusxRuntime, build_runtime};

#[test]
fn runtime_crate_exports_bootstrap_surface() {
    let _ = std::any::type_name::<AppConfig>();
    let _ = std::any::type_name::<ArgusxRuntime>();
    let _ = build_runtime;
}
