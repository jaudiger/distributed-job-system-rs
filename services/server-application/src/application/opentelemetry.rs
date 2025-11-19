use crate::application::APPLICATION_NAME;
use anyhow::Result;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_otlp::WithHttpConfig as _;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

pub struct OpentelemetryHandler {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
}

impl OpentelemetryHandler {
    pub fn new() -> Result<Self> {
        let tracer_provider = Self::create_trace_exporter()?;
        let meter_provider = Self::create_metric_exporter()?;

        let tracer = tracer_provider.tracer(APPLICATION_NAME);

        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(tracing_opentelemetry::MetricsLayer::new(
                meter_provider.clone(),
            ))
            .with(tracing_opentelemetry::OpenTelemetryLayer::new(tracer))
            .try_init()?;

        Ok(Self {
            tracer_provider,
            meter_provider,
        })
    }

    fn create_metric_exporter() -> Result<SdkMeterProvider> {
        let exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
            .with_compression(opentelemetry_otlp::Compression::Gzip)
            .build()?;

        let meter_provider = SdkMeterProvider::builder()
            .with_resource(Self::create_resource())
            .with_periodic_exporter(exporter)
            .build();
        opentelemetry::global::set_meter_provider(meter_provider.clone());

        Ok(meter_provider)
    }

    fn create_trace_exporter() -> Result<SdkTracerProvider> {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
            .with_compression(opentelemetry_otlp::Compression::Gzip)
            .build()?;

        let tracer_provider = SdkTracerProvider::builder()
            .with_resource(Self::create_resource())
            .with_batch_exporter(exporter)
            .build();
        opentelemetry::global::set_tracer_provider(tracer_provider.clone());

        opentelemetry::global::set_text_map_propagator(
            opentelemetry_sdk::propagation::TraceContextPropagator::new(),
        );

        Ok(tracer_provider)
    }

    fn create_resource() -> opentelemetry_sdk::Resource {
        const APPLICATION_VERSION: &str = env!("CARGO_PKG_VERSION");

        opentelemetry_sdk::Resource::builder()
            .with_service_name(super::APPLICATION_NAME)
            .with_attribute(opentelemetry::KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
                APPLICATION_VERSION,
            ))
            .with_detectors(&[
                Box::new(opentelemetry_resource_detectors::OsResourceDetector),
                Box::new(opentelemetry_resource_detectors::ProcessResourceDetector),
                Box::new(opentelemetry_resource_detectors::K8sResourceDetector),
            ])
            .build()
    }
}

impl Drop for OpentelemetryHandler {
    fn drop(&mut self) {
        if let Err(err) = self.tracer_provider.shutdown() {
            eprintln!("{err:?}");
        }
        if let Err(err) = self.meter_provider.shutdown() {
            eprintln!("{err:?}");
        }
    }
}
