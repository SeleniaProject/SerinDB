use anyhow::Result;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::reload::{Handle, Layer as ReloadLayer, ReloadLayer as _};
use tracing::Level;

/// Initialize structured JSON logging with rolling files and runtime log-level reload.
/// `dir` – log directory, `level` – initial log level.
/// Returns a reload handle that can update filter at runtime.
pub fn init(dir: &str, level: Level) -> Result<Handle<EnvFilter, impl tracing::Subscriber + Send + Sync>> {
    let file_appender = RollingFileAppender::new(Rotation::HOURLY, dir, "serindb.log");
    let (reload_env, layer) = ReloadLayer::new(EnvFilter::default().add_directive(level.into()));
    let fmt_layer = fmt::layer()
        .with_writer(file_appender)
        .json()
        .with_current_span(false)
        .with_span_list(false)
        .with_filter(layer);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .init();
    Ok(reload_env)
} 