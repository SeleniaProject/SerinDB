use anyhow::Result;
use opentelemetry::sdk::{trace, Resource};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize OpenTelemetry OTLP exporter and tracing subscriber.
/// Must be called once at application startup.
pub fn init(service_name: &str) -> Result<()> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".into());

    // Build OTLP exporter pipeline.
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_export_config(opentelemetry_otlp::ExportConfig { endpoint, ..Default::default() })
        .with_trace_config(trace::config().with_resource(Resource::new(vec![KeyValue::new("service.name", service_name.to_string())])))
        .install_batch(opentelemetry::runtime::Tokio)?;

    // Build tracing subscriber with OTLP layer + stdout.
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(otel_layer)
        .with(fmt_layer)
        .try_init()?;
    Ok(())
}

/// Shutdown OTLP pipeline gracefully.
pub fn shutdown() {
    opentelemetry::global::shutdown_tracer_provider();
} 