mod session_commands;

use session::manager::SessionManager;
use session::store::ThreadStore;
use session_commands::{
    DesktopSessionState, cancel_thread_turn, create_thread, list_threads,
    resolve_thread_permission, send_message, spawn_session_event_bridge, switch_thread,
};

// Desktop telemetry integration
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = telemetry::TelemetryConfig::from_path("config/telemetry.toml")
        .unwrap_or_else(|_| telemetry::TelemetryConfig::default());
    let runtime = if config.enabled {
        Some(telemetry::init(config)?)
    } else {
        None
    };

    let pool = tauri::async_runtime::block_on(async { sqlx::SqlitePool::connect("sqlite:argusx.db").await })?;
    let store = ThreadStore::new(pool);
    tauri::async_runtime::block_on(async { store.init_schema().await })?;

    let manager = SessionManager::new("default-session".into(), store);
    tauri::async_runtime::block_on(async { manager.initialize().await })?;
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

    if let Some(runtime) = runtime {
        runtime.shutdown(std::time::Duration::from_secs(10))?;
    }

    run_result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn desktop_lib_builds() {
        assert_eq!(2 + 2, 4);
    }
}
