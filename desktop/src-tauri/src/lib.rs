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
