use crate::{manager::SessionManager, thread::ThreadRuntime, types::ThreadRecord};

#[test]
fn session_crate_exports_new_domain_types() {
    let _ = std::any::type_name::<SessionManager>();
    let _ = std::any::type_name::<ThreadRuntime>();
    let _ = std::any::type_name::<ThreadRecord>();
}
