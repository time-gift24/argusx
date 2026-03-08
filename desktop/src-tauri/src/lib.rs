mod session_commands;

use session_commands::{
    DesktopSessionState, cancel_thread_turn, create_thread, list_threads,
    resolve_thread_permission, send_message, spawn_session_event_bridge, switch_thread,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tauri::async_runtime::block_on(runtime::build_runtime())?;
    let manager = runtime.session_manager.clone();
    let session_state = DesktopSessionState::new(manager);
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

    runtime.shutdown(std::time::Duration::from_secs(10))?;
    run_result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn desktop_builds_against_runtime_crate() {
        let _ = std::any::type_name::<runtime::ArgusxRuntime>();
    }
}
