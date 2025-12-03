use crate::tracers::proc::Process;
use futures::SinkExt;
use opentelemetry::KeyValue;
use opentelemetry::global::ObjectSafeTracer;
use opentelemetry::metrics::MeterProvider;
use opentelemetry_otlp::{
    HttpExporterBuilder, MetricExporter, Protocol, WithExportConfig, WithHttpConfig,
};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, trace, error};

fn init_tracing_subscriber() {
    // Initialize tracing subscriber to capture logs from both your app and OpenTelemetry internals
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    // Default filter: show INFO for your app, DEBUG for OpenTelemetry
                    tracing_subscriber::EnvFilter::new(
                        "async_tests=debug,\
                         opentelemetry=debug,\
                         opentelemetry_sdk=debug,\
                         opentelemetry_otlp=debug,\
                         opentelemetry_http=trace"
                    )
                })
        )
        .with_target(true)      // Show the module path
        .with_thread_ids(true)  // Show thread IDs
        .with_line_number(true) // Show line numbers
        .init();
}

fn init_otel_resource() -> opentelemetry_sdk::Resource {
    debug!("Initializing OTEL resource");
    let otlp_resource_detected = opentelemetry_sdk::Resource::builder()
        .with_detector(Box::new(
            opentelemetry_sdk::resource::SdkProvidedResourceDetector,
        ))
        .with_detector(Box::new(
            opentelemetry_sdk::resource::EnvResourceDetector::new(),
        ))
        .with_detector(Box::new(
            opentelemetry_sdk::resource::TelemetryResourceDetector,
        ))
        .with_service_name("rust-agents-test");

    let resource = otlp_resource_detected.build();
    debug!("OTEL resource initialized with {} attributes", resource.len());
    trace!("Resource attributes: {:?}", resource);
    resource
}

// ************************************ METRICS ************************************
fn init_metrics(
    // config: &Config,
    resource: opentelemetry_sdk::Resource,
) -> opentelemetry_sdk::metrics::SdkMeterProvider {
    debug!("Initializing metrics exporter");
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318".to_string());
    let headers = std::env::var("OTEL_EXPORTER_OTLP_HEADERS")
        .map(|h| parse_headers(&h))
        .unwrap_or_default();
    // let protocol = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL")
    //     .unwrap_or_else(|_| "http/protobuf".to_string());

    info!("OTEL_EXPORTER_OTLP_ENDPOINT: {}", endpoint);
    debug!("OTEL_EXPORTER_OTLP_HEADERS: {:?}", headers);

    let mut meter_provider_builder = opentelemetry_sdk::metrics::SdkMeterProvider::builder();
        debug!("Building OTLP HTTP exporter with protocol: HttpBinary");
        let otlp_exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_endpoint(&endpoint)
            .with_headers(headers)
            .with_protocol(Protocol::HttpBinary)
            // .with_tonic()
            .build()
            .expect("Failed to build OTLP exporter");

        debug!("Creating PeriodicReader with 10s interval");
        let otlp_reader = opentelemetry_sdk::metrics::PeriodicReader::builder(otlp_exporter)
            .with_interval(Duration::from_secs(10))
            .build();

        meter_provider_builder = meter_provider_builder.with_reader(otlp_reader);

    debug!("Building meter provider");
    let meter_provider = meter_provider_builder.with_resource(resource).build();
    info!("Metrics exporter initialized successfully");
    meter_provider
}

fn parse_headers(header_str: &str) -> HashMap<String, String> {
    header_str
        .split(',')
        .filter_map(|pair| {
            let mut parts = pair.trim().split('=');
            Some((
                parts.next()?.to_string(),
                parts.next()?.to_string(),
            ))
        })
        .collect()
}


pub async fn exporter(ct: CancellationToken, mut input: tokio::sync::mpsc::Receiver<Vec<Process>>) {
    info!("Starting OTEL metrics exporter");
    init_tracing_subscriber();

    debug!("OTEL environment variables:");
    std::env::vars()
        .filter(|(key, _)| key.starts_with("OTEL_"))
        .for_each(|(key, value)| debug!("  {} = {}", key, value));

    let resource = init_otel_resource();
    let provider = init_metrics(resource);

    let meter = provider.meter("metric_exporter");

    // TODO: observable counter to remove process metrics when they are no longer needed
    let proc_cpu_time = meter.u64_counter("process.cpu.time").build();
    debug!("Created process.cpu.time counter");

    let mut batch_count = 0u64;

    loop {
        tokio::select!(
            _ = ct.cancelled() => {
                info!("Cancellation requested, shutting down metrics exporter");
                if let Err(err) = provider.shutdown() {
                    error!("Error shutting down provider: {:?}", err);
                } else {
                    debug!("Provider shut down successfully");
                }
                return;
            }
            Some(metrics) = input.recv() => {
                batch_count += 1;
                debug!("Received batch #{} with {} processes", batch_count, metrics.len());

                metrics.iter().take(3).for_each(|proc| {
                    trace!("Recording metric for PID {} ({}): cpu_time={}",
                        proc.pid, proc.exe, proc.cpu_time);

                    proc_cpu_time.add(metrics.len() as u64, &[
                        // this IS NOT compliant with the OTEL standard, as the following attributes
                        // are resource attributes, not metric attributes
                        KeyValue::new("process.pid", proc.pid.to_string()),
                        KeyValue::new("process.executable.path", proc.exe.to_string()),
                        KeyValue::new("process.cpu.time", proc.cpu_time.to_string())
                    ]);
                });
            },
        )
    }
}
