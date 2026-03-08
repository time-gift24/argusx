use tauri::Manager;

pub mod chat;

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

    let run_result = tauri::Builder::default()
        .setup(|app| {
            app.manage(chat::commands::ChatState::from_app(app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            chat::commands::start_conversation,
            chat::commands::continue_conversation,
            chat::commands::cancel_conversation,
            chat::commands::create_conversation_thread,
            chat::commands::list_conversation_threads,
            chat::commands::switch_conversation_thread,
            chat::commands::create_conversation_checkpoint,
            chat::commands::restore_conversation_checkpoint,
            chat::commands::restart_conversation
        ])
        .plugin(tauri_plugin_opener::init())
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
