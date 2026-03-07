// Desktop telemetry integration
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app_handle| {
            // Initialize telemetry runtime
            let config = telemetry::TelemetryConfig::default();
            if config.enabled {
                match telemetry::init(config) {
                    Ok(_runtime) => {
                        tracing::info!(
                            event_name = "telemetry_initialized"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            event_name = "telemetry_init_failed",
                            error = e.to_string().as_str()
                        );
                    }
                }
            }
            Ok(())
        })
        .on_window_event(|_app, event| {
            // Emit window closed event for telemetry
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                tracing::info!(
                    event_name = "window_closed"
                );
            }
        })
        .run(tauri::generate_context!())?;
    Ok(())
}
