pub mod chat;
pub mod provider_settings;
mod session_commands;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

use session_commands::{
    cancel_thread_turn, create_thread, list_threads, resolve_thread_permission, send_message,
    spawn_session_event_bridge, switch_thread, DesktopSessionState,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), BoxError> {
    let runtime = tauri::async_runtime::block_on(runtime::build_runtime())?;
    let manager = runtime.session_manager.clone();
    let session_state = DesktopSessionState::new(manager).map_err(|err| -> BoxError { Box::new(err) })?;
    let bridge_manager = session_state.manager.clone();

    let run_result = tauri::Builder::default()
        .manage(session_state)
        .setup(move |app| {
            spawn_session_event_bridge(app.handle().clone(), bridge_manager.clone());
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            create_thread,
            list_threads,
            switch_thread,
            send_message,
            resolve_thread_permission,
            cancel_thread_turn,
            chat::commands::start_turn,
            chat::commands::cancel_turn,
            chat::commands::load_active_chat_thread,
            chat::commands::resolve_turn_permission,
            provider_settings::commands::list_provider_profiles,
            provider_settings::commands::save_provider_profile,
            provider_settings::commands::delete_provider_profile,
            provider_settings::commands::set_default_provider_profile,
            provider_settings::commands::test_provider_profile
        ])
        .on_window_event(|_app, event| {
            // Emit window closed event for telemetry
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                tracing::info!(event_name = "window_closed");
            }
        })
        .run(tauri::generate_context!());

    if let Err(ref err) = run_result {
        tracing::error!(event_name = "tauri_run_error", error = %err);
    }

    let shutdown_result = runtime
        .shutdown(std::time::Duration::from_secs(10))
        .map_err(Into::into);
    if let Err(ref err) = shutdown_result {
        tracing::error!(event_name = "runtime_shutdown_error", error = %err);
    }

    finish_run(run_result.map_err(Into::into), shutdown_result)
}

fn finish_run(
    run_result: Result<(), BoxError>,
    shutdown_result: Result<(), BoxError>,
) -> Result<(), BoxError> {
    match (run_result, shutdown_result) {
        (Err(run_err), _) => Err(run_err),
        (Ok(()), Err(shutdown_err)) => Err(shutdown_err),
        (Ok(()), Ok(())) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Error;

    use super::finish_run;

    #[test]
    fn desktop_builds_against_runtime_crate() {
        let _ = std::any::type_name::<runtime::ArgusxRuntime>();
    }

    #[test]
    fn finish_run_preserves_tauri_error_when_shutdown_also_fails() {
        let err = finish_run(
            Err(Box::new(Error::other("tauri run failed"))),
            Err(Box::new(Error::other("telemetry shutdown failed"))),
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "tauri run failed");
    }

    #[test]
    fn finish_run_returns_shutdown_error_after_successful_run() {
        let err = finish_run(
            Ok(()),
            Err(Box::new(Error::other("telemetry shutdown failed"))),
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "telemetry shutdown failed");
    }
}
