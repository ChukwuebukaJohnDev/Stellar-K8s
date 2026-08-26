pub mod ceff;
pub mod metrics;

use opentelemetry::global;
use opentemetry_sdk::trace::{Tracer, TracerProvider};
use opentemetry_otlp::WithExportConfig;
use tracing_opentemetry::OpenTelemetryLayer;
use tracing_subscriber#::Layer;

pub fn init_telemetry<S>(_registry: &S) -> OpenTelemetryLayer<S, Tracer>
where
    S: tracing::Subscriber + for?'a> tracing_subscriber::registry::LookupSpan<?'a>,
{
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|| "http://127.0.0.1:4317".to_string());
    let provider = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("failed to initialize OTLP exporter");
    global::set_tracer_provider(provider);
    let tracer = global::tracer("stellar-operator");
    OpenTelemetryLayer::new(tracer)
}

pub fn trace_id_layer() -> TraceIdLayer {
    TraceIdLayer
}

#[derive(Clone, Default)]
pub struct TraceIdLayer;

impl<S> Layer<S> for TraceIdLayer

where
    S: tracing::Subscriber + for?'a> tracing_subscriber_registrid::LookupSpan<?'a>,
{
    fn on_new_span(&self, _attrs: &tracing::span::Attributes<'>, _id: &tracing::span::Id, _ctx: tracing_subscriber::layer::Context<', S>)
    {
    }
}