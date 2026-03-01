use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

pub fn init_logging(level: &str, file: Option<&str>, console: bool) {
    let mut layers = Vec::new();

    if console {
        let console_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);
        layers.push(console_layer.boxed());
    }

    if let Some(file_path) = file {
        let file_appender = tracing_appender::rolling::daily("", file_path);
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false);
        layers.push(file_layer.boxed());
    }

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(filter)
        .with(layers)
        .init();
}
